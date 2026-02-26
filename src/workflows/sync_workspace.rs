//! Shared sync workflow for CLI and TUI.

use crate::auth::{get_auth_for_provider, AuthResult};
use crate::cache::{CacheManager, DiscoveryCache};
use crate::config::{Config, WorkspaceConfig};
use crate::discovery::DiscoveryOrchestrator;
use crate::errors::{AppError, Result};
use crate::git::{CloneOptions, ShellGit};
use crate::operations::clone::{
    CloneManager, CloneManagerOptions, CloneProgress, MAX_CONCURRENCY, MIN_CONCURRENCY,
};
use crate::operations::sync::{
    LocalRepo, SyncManager, SyncManagerOptions, SyncMode, SyncProgress, SyncResult,
};
use crate::provider::{create_provider, DiscoveryProgress};
use crate::types::{ActionPlan, OpSummary, OwnedRepo};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;

/// Request data used to prepare a workspace sync plan.
pub struct SyncWorkspaceRequest<'a> {
    pub config: &'a Config,
    pub workspace: &'a WorkspaceConfig,
    pub refresh: bool,
    pub skip_uncommitted: bool,
    pub pull: bool,
    pub concurrency_override: Option<usize>,
    pub create_base_path: bool,
}

/// Prepared sync workflow context.
pub struct PreparedSyncWorkspace {
    pub workspace: WorkspaceConfig,
    pub auth: AuthResult,
    pub repos: Vec<OwnedRepo>,
    pub used_cache: bool,
    pub cache_age_secs: Option<u64>,
    pub base_path: PathBuf,
    pub structure: String,
    pub provider_name: String,
    pub provider_prefer_ssh: bool,
    pub skip_uncommitted: bool,
    pub sync_mode: SyncMode,
    pub requested_concurrency: usize,
    pub effective_concurrency: usize,
    pub plan: ActionPlan,
    pub to_sync: Vec<LocalRepo>,
    pub skipped_sync: Vec<(OwnedRepo, String)>,
    pub clone_options: CloneOptions,
}

/// Execution outcome for a prepared sync workflow.
pub struct SyncExecutionOutcome {
    pub clone_summary: Option<OpSummary>,
    pub sync_summary: Option<OpSummary>,
    pub sync_results: Vec<SyncResult>,
}

/// Prepare workspace sync data: authenticate, discover, plan and resolve options.
pub async fn prepare_sync_workspace(
    request: SyncWorkspaceRequest<'_>,
    discovery_progress: &dyn DiscoveryProgress,
) -> Result<PreparedSyncWorkspace> {
    let provider_entry = request.workspace.provider.to_provider_entry();

    // Authenticate and build provider
    let auth = get_auth_for_provider(&provider_entry)?;
    let provider = create_provider(&provider_entry, &auth.token)?;

    // Build orchestrator from workspace + global config
    let mut filters = request.workspace.filters.clone();
    if !request.workspace.orgs.is_empty() {
        filters.orgs = request.workspace.orgs.clone();
    }
    filters.exclude_repos = request.workspace.exclude_repos.clone();

    let structure = request
        .workspace
        .structure
        .clone()
        .unwrap_or_else(|| request.config.structure.clone());
    let orchestrator = DiscoveryOrchestrator::new(filters, structure.clone());

    // Discover repos (cache first unless refresh)
    let mut repos = Vec::new();
    let mut used_cache = false;
    let mut cache_age_secs = None;

    if !request.refresh {
        if let Ok(cache_manager) = CacheManager::for_workspace(&request.workspace.name) {
            if let Ok(Some(cache)) = cache_manager.load() {
                used_cache = true;
                cache_age_secs = Some(cache.age_secs());
                for provider_repos in cache.repos.values() {
                    repos.extend(provider_repos.clone());
                }

                // Surface cached counts through the existing progress interface
                // so callers can keep one rendering path.
                let org_count = repos
                    .iter()
                    .map(|r| r.owner.clone())
                    .collect::<HashSet<_>>()
                    .len();
                discovery_progress.on_orgs_discovered(org_count);
                let mut by_org: HashMap<String, usize> = HashMap::new();
                for repo in &repos {
                    *by_org.entry(repo.owner.clone()).or_insert(0) += 1;
                }
                for (org, count) in by_org {
                    discovery_progress.on_org_complete(&org, count);
                }
            }
        }
    }

    if repos.is_empty() {
        repos = orchestrator
            .discover(provider.as_ref(), discovery_progress)
            .await
            .map_err(AppError::Provider)?;

        if let Ok(cache_manager) = CacheManager::for_workspace(&request.workspace.name) {
            let provider_label = provider_entry
                .name
                .clone()
                .unwrap_or_else(|| provider_entry.kind.to_string());
            let mut repos_by_provider = HashMap::new();
            repos_by_provider.insert(provider_label, repos.clone());
            let cache =
                DiscoveryCache::new(auth.username.clone().unwrap_or_default(), repos_by_provider);
            let _ = cache_manager.save(&cache);
        }
    }

    let base_path = request.workspace.expanded_base_path();
    if !base_path.exists() {
        if request.create_base_path {
            std::fs::create_dir_all(&base_path).map_err(|e| {
                AppError::path(format!(
                    "Failed to create base directory '{}': {}",
                    base_path.display(),
                    e
                ))
            })?;
        } else {
            return Err(AppError::config(format!(
                "Base path does not exist: {}",
                base_path.display()
            )));
        }
    }

    let provider_name = provider_entry.kind.to_string().to_lowercase();
    let git = ShellGit::new();
    let plan = orchestrator.plan_clone(&base_path, repos.clone(), &provider_name, &git);
    let (to_sync, skipped_sync) = orchestrator.plan_sync(
        &base_path,
        repos.clone(),
        &provider_name,
        &git,
        request.skip_uncommitted,
    );

    let requested_concurrency = request
        .concurrency_override
        .or(request.workspace.concurrency)
        .unwrap_or(request.config.concurrency);
    let effective_concurrency = requested_concurrency.clamp(MIN_CONCURRENCY, MAX_CONCURRENCY);

    let sync_mode = if request.pull {
        SyncMode::Pull
    } else {
        match request
            .workspace
            .sync_mode
            .unwrap_or(request.config.sync_mode)
        {
            crate::config::SyncMode::Pull => SyncMode::Pull,
            crate::config::SyncMode::Fetch => SyncMode::Fetch,
        }
    };

    let clone_options = CloneOptions {
        depth: request
            .workspace
            .clone_options
            .as_ref()
            .map(|c| c.depth)
            .unwrap_or(request.config.clone.depth),
        branch: request
            .workspace
            .clone_options
            .as_ref()
            .and_then(|c| {
                if c.branch.is_empty() {
                    None
                } else {
                    Some(c.branch.clone())
                }
            })
            .or_else(|| {
                if request.config.clone.branch.is_empty() {
                    None
                } else {
                    Some(request.config.clone.branch.clone())
                }
            }),
        recurse_submodules: request
            .workspace
            .clone_options
            .as_ref()
            .map(|c| c.recurse_submodules)
            .unwrap_or(request.config.clone.recurse_submodules),
    };

    Ok(PreparedSyncWorkspace {
        workspace: request.workspace.clone(),
        auth,
        repos,
        used_cache,
        cache_age_secs,
        base_path,
        structure,
        provider_name,
        provider_prefer_ssh: provider_entry.prefer_ssh,
        skip_uncommitted: request.skip_uncommitted,
        sync_mode,
        requested_concurrency,
        effective_concurrency,
        plan,
        to_sync,
        skipped_sync,
        clone_options,
    })
}

/// Execute clone + sync phases for a prepared workspace plan.
pub async fn execute_prepared_sync(
    prepared: &PreparedSyncWorkspace,
    dry_run: bool,
    clone_progress: Arc<dyn CloneProgress>,
    sync_progress: Arc<dyn SyncProgress>,
) -> SyncExecutionOutcome {
    if dry_run {
        return SyncExecutionOutcome {
            clone_summary: None,
            sync_summary: None,
            sync_results: Vec::new(),
        };
    }

    let mut clone_summary = None;
    let mut sync_summary = None;
    let mut sync_results = Vec::new();

    if !prepared.plan.to_clone.is_empty() {
        let clone_options = CloneManagerOptions::new()
            .with_concurrency(prepared.effective_concurrency)
            .with_clone_options(prepared.clone_options.clone())
            .with_structure(prepared.structure.clone())
            .with_ssh(prepared.provider_prefer_ssh);

        let manager = CloneManager::new(ShellGit::new(), clone_options);
        let (summary, _results) = manager
            .clone_repos(
                &prepared.base_path,
                prepared.plan.to_clone.clone(),
                &prepared.provider_name,
                clone_progress,
            )
            .await;
        clone_summary = Some(summary);
    }

    if !prepared.to_sync.is_empty() {
        let sync_options = SyncManagerOptions::new()
            .with_concurrency(prepared.effective_concurrency)
            .with_mode(prepared.sync_mode)
            .with_skip_uncommitted(prepared.skip_uncommitted);

        let manager = SyncManager::new(ShellGit::new(), sync_options);
        let (summary, results) = manager
            .sync_repos(prepared.to_sync.clone(), sync_progress)
            .await;
        sync_summary = Some(summary);
        sync_results = results;
    }

    SyncExecutionOutcome {
        clone_summary,
        sync_summary,
        sync_results,
    }
}
