//! Sync command handler.
//!
//! Combined operation: discover repos → clone new ones → fetch/pull existing ones.

use super::warn_if_concurrency_capped;
use crate::auth::get_auth_for_provider;
use crate::cache::{CacheManager, DiscoveryCache};
use crate::cli::SyncCmdArgs;
use crate::config::{Config, WorkspaceManager};
use crate::discovery::DiscoveryOrchestrator;
use crate::errors::{AppError, Result};
use crate::git::{CloneOptions, ShellGit};
use crate::operations::clone::{CloneManager, CloneManagerOptions, CloneProgress};
use crate::operations::sync::{SyncManager, SyncManagerOptions, SyncMode, SyncProgress};
use crate::output::{
    format_count, CloneProgressBar, DiscoveryProgressBar, Output, SyncProgressBar, Verbosity,
};
use crate::provider::create_provider;
use std::sync::Arc;

/// Sync repositories for a workspace.
pub async fn run(args: &SyncCmdArgs, config: &Config, output: &Output) -> Result<()> {
    let verbosity = if output.is_json() {
        Verbosity::Quiet
    } else {
        output.verbosity()
    };

    // Resolve workspace
    let mut workspace = WorkspaceManager::resolve(args.workspace.as_deref(), config)?;
    let provider_entry = workspace.provider.to_provider_entry();

    // Authenticate
    output.info("Authenticating...");
    let auth = get_auth_for_provider(&provider_entry)?;
    output.verbose(&format!(
        "Authenticated as {:?} via {}",
        auth.username, auth.method
    ));

    // Create provider
    let provider = create_provider(&provider_entry, &auth.token)?;

    // Build filters from workspace config
    let mut filters = workspace.filters.clone();
    if !workspace.orgs.is_empty() {
        filters.orgs = workspace.orgs.clone();
    }
    filters.exclude_repos = workspace.exclude_repos.clone();

    let structure = workspace
        .structure
        .clone()
        .unwrap_or_else(|| config.structure.clone());
    let orchestrator = DiscoveryOrchestrator::new(filters, structure.clone());

    // Discover repos (with cache support)
    let mut repos = Vec::new();
    let use_cache = !args.refresh;

    if use_cache {
        if let Ok(cache_manager) = CacheManager::new() {
            if let Ok(Some(cache)) = cache_manager.load() {
                output.verbose(&format!(
                    "Using cached discovery ({} repos, {} seconds old)",
                    cache.repo_count,
                    cache.age_secs()
                ));
                for provider_repos in cache.repos.values() {
                    repos.extend(provider_repos.clone());
                }
            }
        }
    }

    if repos.is_empty() {
        output.info("Discovering repositories...");
        let progress_bar = DiscoveryProgressBar::new(verbosity);
        repos = orchestrator
            .discover(provider.as_ref(), &progress_bar)
            .await?;
        progress_bar.finish();

        // Save to cache
        if let Ok(cache_manager) = CacheManager::new() {
            let provider_name = provider_entry
                .name
                .clone()
                .unwrap_or_else(|| provider_entry.kind.to_string());
            let mut repos_by_provider = std::collections::HashMap::new();
            repos_by_provider.insert(provider_name, repos.clone());
            let cache =
                DiscoveryCache::new(auth.username.clone().unwrap_or_default(), repos_by_provider);
            if let Err(e) = cache_manager.save(&cache) {
                output.verbose(&format!("Warning: Failed to save discovery cache: {}", e));
            }
        }
    }

    if repos.is_empty() {
        output.warn("No repositories found matching filters");
        return Ok(());
    }

    output.info(&format_count(repos.len(), "repositories discovered"));

    // Ensure base path exists
    let base_path = workspace.expanded_base_path();
    if !base_path.exists() {
        std::fs::create_dir_all(&base_path)
            .map_err(|e| AppError::path(format!("Failed to create base directory: {}", e)))?;
    }

    // Plan: which repos to clone (new) and which to sync (existing)
    let git = ShellGit::new();
    let provider_name = provider_entry.kind.to_string().to_lowercase();
    let plan = orchestrator.plan_clone(&base_path, repos.clone(), &provider_name, &git);

    let concurrency = args
        .concurrency
        .or(workspace.concurrency)
        .unwrap_or(config.concurrency);
    let effective_concurrency = warn_if_concurrency_capped(concurrency, output);
    let skip_dirty = !args.no_skip_dirty;

    // Phase 1: Clone new repos
    let had_clones = !plan.to_clone.is_empty();
    if had_clones {
        if args.dry_run {
            output.info(&format!(
                "Would clone {} new repositories:",
                plan.to_clone.len()
            ));
            for repo in &plan.to_clone {
                println!("  + {}", repo.full_name());
            }
        } else {
            output.info(&format_count(
                plan.to_clone.len(),
                "new repositories to clone",
            ));

            let clone_options = CloneOptions {
                depth: workspace
                    .clone_options
                    .as_ref()
                    .map(|c| c.depth)
                    .unwrap_or(config.clone.depth),
                branch: workspace
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
                        if config.clone.branch.is_empty() {
                            None
                        } else {
                            Some(config.clone.branch.clone())
                        }
                    }),
                recurse_submodules: workspace
                    .clone_options
                    .as_ref()
                    .map(|c| c.recurse_submodules)
                    .unwrap_or(config.clone.recurse_submodules),
            };

            let manager_options = CloneManagerOptions::new()
                .with_concurrency(effective_concurrency)
                .with_clone_options(clone_options)
                .with_structure(structure.clone())
                .with_ssh(provider_entry.prefer_ssh);

            let manager = CloneManager::new(ShellGit::new(), manager_options);
            let progress = Arc::new(CloneProgressBar::new(plan.to_clone.len(), verbosity));
            let progress_dyn: Arc<dyn CloneProgress> = progress.clone();
            let (summary, _results) = manager
                .clone_repos(&base_path, plan.to_clone, &provider_name, progress_dyn)
                .await;
            progress.finish(summary.success, summary.failed, summary.skipped);

            if summary.has_failures() {
                output.warn(&format!("{} repositories failed to clone", summary.failed));
            } else {
                output.success(&format!("Cloned {} new repositories", summary.success));
            }
        }
    }

    // Phase 2: Sync existing repos
    let sync_mode = if args.pull {
        SyncMode::Pull
    } else {
        match workspace.sync_mode.unwrap_or(config.sync_mode) {
            crate::config::SyncMode::Pull => SyncMode::Pull,
            crate::config::SyncMode::Fetch => SyncMode::Fetch,
        }
    };
    let operation = if sync_mode == SyncMode::Pull {
        "Pull"
    } else {
        "Fetch"
    };

    // Re-plan sync for existing repos
    let (to_sync, skipped) =
        orchestrator.plan_sync(&base_path, repos, &provider_name, &git, skip_dirty);

    if !to_sync.is_empty() {
        if args.dry_run {
            output.info(&format!(
                "Would {} {} existing repositories:",
                operation.to_lowercase(),
                to_sync.len()
            ));
            for repo in &to_sync {
                println!("  ~ {}", repo.repo.full_name());
            }
        } else {
            output.info(&format_count(
                to_sync.len(),
                &format!("existing repositories to {}", operation.to_lowercase()),
            ));
            if !skipped.is_empty() {
                output.verbose(&format_count(skipped.len(), "repositories skipped"));
            }

            let manager_options = SyncManagerOptions::new()
                .with_concurrency(effective_concurrency)
                .with_mode(sync_mode)
                .with_skip_dirty(skip_dirty);

            let manager = SyncManager::new(ShellGit::new(), manager_options);
            let progress = Arc::new(SyncProgressBar::new(to_sync.len(), verbosity, operation));
            let progress_dyn: Arc<dyn SyncProgress> = progress.clone();
            let (summary, results) = manager.sync_repos(to_sync, progress_dyn).await;
            progress.finish(summary.success, summary.failed, summary.skipped);

            let with_updates = results.iter().filter(|r| r.had_updates).count();

            if summary.has_failures() {
                output.warn(&format!(
                    "{} of {} repositories failed to {}",
                    summary.failed,
                    summary.total(),
                    operation.to_lowercase()
                ));
            } else {
                output.success(&format!(
                    "{}ed {} repositories ({} with updates)",
                    operation, summary.success, with_updates
                ));
            }
        }
    } else if !had_clones {
        output.success("All repositories are up to date");
    }

    // Update last_synced
    if !args.dry_run {
        workspace.last_synced = Some(chrono::Utc::now().to_rfc3339());
        if let Err(e) = WorkspaceManager::save(&workspace) {
            output.verbose(&format!("Warning: Failed to update last_synced: {}", e));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    // Sync command orchestrates workspace -> auth -> provider -> discovery -> clone + sync.
    // Unit tests are not feasible because `run()` requires real credentials.
    //
    // Component-level tests exist in:
    // - src/operations/clone.rs (CloneManager)
    // - src/operations/sync.rs (SyncManager)
    // - src/discovery/mod.rs (DiscoveryOrchestrator)
    // - src/config/workspace.rs (WorkspaceConfig)
    // - src/config/workspace_manager.rs (WorkspaceManager)
    //
    // Integration coverage: tests/integration_test.rs
}
