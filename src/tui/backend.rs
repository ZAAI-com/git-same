//! Backend integration — bridges TUI with existing async command handlers.
//!
//! Provides channel-based progress adapters and spawn functions for operations.

use std::path::Path;
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;

use crate::auth::get_auth_for_provider;
use crate::config::{Config, WorkspaceConfig};
use crate::discovery::DiscoveryOrchestrator;
use crate::git::{CloneOptions, FetchResult, GitOperations, PullResult, ShellGit};
use crate::operations::clone::{CloneManager, CloneManagerOptions, CloneProgress};
use crate::operations::sync::{SyncManager, SyncManagerOptions, SyncMode, SyncProgress};
use crate::provider::{create_provider, DiscoveryProgress};
use crate::types::{OpSummary, OwnedRepo};

use super::app::{App, Operation, RepoEntry};
use super::event::{AppEvent, BackendMessage};

// -- Progress adapters that send events to the TUI via channels --

struct TuiDiscoveryProgress {
    tx: UnboundedSender<AppEvent>,
}

impl DiscoveryProgress for TuiDiscoveryProgress {
    fn on_orgs_discovered(&self, count: usize) {
        let _ = self
            .tx
            .send(AppEvent::Backend(BackendMessage::OrgsDiscovered(count)));
    }

    fn on_org_started(&self, org_name: &str) {
        let _ = self.tx.send(AppEvent::Backend(BackendMessage::OrgStarted(
            org_name.to_string(),
        )));
    }

    fn on_org_complete(&self, org_name: &str, repo_count: usize) {
        let _ = self.tx.send(AppEvent::Backend(BackendMessage::OrgComplete(
            org_name.to_string(),
            repo_count,
        )));
    }

    fn on_personal_repos_started(&self) {}

    fn on_personal_repos_complete(&self, _count: usize) {}

    fn on_error(&self, message: &str) {
        let _ = self
            .tx
            .send(AppEvent::Backend(BackendMessage::DiscoveryError(
                message.to_string(),
            )));
    }
}

struct TuiCloneProgress {
    tx: UnboundedSender<AppEvent>,
}

impl CloneProgress for TuiCloneProgress {
    fn on_start(&self, repo: &OwnedRepo, _index: usize, _total: usize) {
        let _ = self.tx.send(AppEvent::Backend(BackendMessage::RepoStarted {
            repo_name: repo.full_name().to_string(),
        }));
    }

    fn on_complete(&self, repo: &OwnedRepo, _index: usize, _total: usize) {
        let _ = self
            .tx
            .send(AppEvent::Backend(BackendMessage::RepoProgress {
                repo_name: repo.full_name().to_string(),
                success: true,
                skipped: false,
                message: "cloned".to_string(),
                had_updates: true,
                is_clone: true,
                new_commits: None,
                skip_reason: None,
            }));
    }

    fn on_error(&self, repo: &OwnedRepo, error: &str, _index: usize, _total: usize) {
        let _ = self
            .tx
            .send(AppEvent::Backend(BackendMessage::RepoProgress {
                repo_name: repo.full_name().to_string(),
                success: false,
                skipped: false,
                message: error.to_string(),
                had_updates: false,
                is_clone: true,
                new_commits: None,
                skip_reason: None,
            }));
    }

    fn on_skip(&self, repo: &OwnedRepo, reason: &str, _index: usize, _total: usize) {
        let _ = self
            .tx
            .send(AppEvent::Backend(BackendMessage::RepoProgress {
                repo_name: repo.full_name().to_string(),
                success: true,
                skipped: true,
                message: format!("skipped: {}", reason),
                had_updates: false,
                is_clone: true,
                new_commits: None,
                skip_reason: Some(reason.to_string()),
            }));
    }
}

struct TuiSyncProgress {
    tx: UnboundedSender<AppEvent>,
}

impl SyncProgress for TuiSyncProgress {
    fn on_start(&self, repo: &OwnedRepo, _path: &Path, _index: usize, _total: usize) {
        let _ = self.tx.send(AppEvent::Backend(BackendMessage::RepoStarted {
            repo_name: repo.full_name().to_string(),
        }));
    }

    fn on_fetch_complete(
        &self,
        repo: &OwnedRepo,
        result: &FetchResult,
        _index: usize,
        _total: usize,
    ) {
        let status = if result.updated {
            "updated"
        } else {
            "up to date"
        };
        let _ = self
            .tx
            .send(AppEvent::Backend(BackendMessage::RepoProgress {
                repo_name: repo.full_name().to_string(),
                success: true,
                skipped: false,
                message: status.to_string(),
                had_updates: result.updated,
                is_clone: false,
                new_commits: result.new_commits,
                skip_reason: None,
            }));
    }

    fn on_pull_complete(
        &self,
        repo: &OwnedRepo,
        result: &PullResult,
        _index: usize,
        _total: usize,
    ) {
        let status = if result.fast_forward {
            "fast-forward"
        } else {
            "pulled"
        };
        let _ = self
            .tx
            .send(AppEvent::Backend(BackendMessage::RepoProgress {
                repo_name: repo.full_name().to_string(),
                success: result.success,
                skipped: false,
                message: status.to_string(),
                had_updates: result.success,
                is_clone: false,
                new_commits: None,
                skip_reason: None,
            }));
    }

    fn on_error(&self, repo: &OwnedRepo, error: &str, _index: usize, _total: usize) {
        let _ = self
            .tx
            .send(AppEvent::Backend(BackendMessage::RepoProgress {
                repo_name: repo.full_name().to_string(),
                success: false,
                skipped: false,
                message: error.to_string(),
                had_updates: false,
                is_clone: false,
                new_commits: None,
                skip_reason: None,
            }));
    }

    fn on_skip(&self, repo: &OwnedRepo, reason: &str, _index: usize, _total: usize) {
        let _ = self
            .tx
            .send(AppEvent::Backend(BackendMessage::RepoProgress {
                repo_name: repo.full_name().to_string(),
                success: true,
                skipped: true,
                message: format!("skipped: {}", reason),
                had_updates: false,
                is_clone: false,
                new_commits: None,
                skip_reason: Some(reason.to_string()),
            }));
    }
}

// -- Spawn functions --

/// Spawn an async task to fetch recent commits for a repo (post-sync deep dive).
pub fn spawn_commit_fetch(
    repo_path: std::path::PathBuf,
    repo_name: String,
    tx: UnboundedSender<AppEvent>,
) {
    tokio::spawn(async move {
        let commits = tokio::task::spawn_blocking(move || {
            let git = ShellGit::new();
            git.recent_commits(&repo_path, 30).unwrap_or_default()
        })
        .await
        .unwrap_or_default();

        let _ = tx.send(AppEvent::Backend(BackendMessage::RepoCommitLog {
            repo_name,
            commits,
        }));
    });
}

/// Spawn commit fetches for multiple repos (aggregate changelog).
pub fn spawn_changelog_fetch(
    repos: Vec<(String, std::path::PathBuf)>,
    tx: UnboundedSender<AppEvent>,
) {
    for (repo_name, repo_path) in repos {
        let tx = tx.clone();
        tokio::spawn(async move {
            let commits = tokio::task::spawn_blocking(move || {
                let git = ShellGit::new();
                git.recent_commits(&repo_path, 30).unwrap_or_default()
            })
            .await
            .unwrap_or_default();

            let _ = tx.send(AppEvent::Backend(BackendMessage::RepoCommitLog {
                repo_name,
                commits,
            }));
        });
    }
}

/// Spawn a backend operation as a Tokio task.
pub fn spawn_operation(operation: Operation, app: &App, tx: UnboundedSender<AppEvent>) {
    let config = app.config.clone();
    let workspace = app.active_workspace.clone();
    let sync_pull = app.sync_pull;

    match operation {
        Operation::Sync => {
            tokio::spawn(async move {
                run_sync_operation(config, workspace, tx, sync_pull).await;
            });
        }
        Operation::Status => {
            let workspace = app.active_workspace.clone();
            let config = app.config.clone();
            tokio::spawn(async move {
                run_status_scan(config, workspace, tx).await;
            });
        }
    }
}

/// Combined sync operation: discover → clone new → fetch/pull existing.
async fn run_sync_operation(
    config: Config,
    workspace: Option<WorkspaceConfig>,
    tx: UnboundedSender<AppEvent>,
    pull_mode: bool,
) {
    let workspace = match workspace {
        Some(ws) => ws,
        None => {
            let _ = tx.send(AppEvent::Backend(BackendMessage::OperationError(
                "No workspace selected. Run 'gisa setup' to configure one.".to_string(),
            )));
            return;
        }
    };

    let base_path = workspace.expanded_base_path();
    let provider_entry = workspace.provider.to_provider_entry();

    // Authenticate
    let auth = match get_auth_for_provider(&provider_entry) {
        Ok(a) => a,
        Err(e) => {
            let _ = tx.send(AppEvent::Backend(BackendMessage::OperationError(format!(
                "Auth failed: {}",
                e
            ))));
            return;
        }
    };

    // Create provider
    let provider = match create_provider(&provider_entry, &auth.token) {
        Ok(p) => p,
        Err(e) => {
            let _ = tx.send(AppEvent::Backend(BackendMessage::OperationError(format!(
                "Provider error: {}",
                e
            ))));
            return;
        }
    };

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

    // Discover
    let discovery_progress = TuiDiscoveryProgress { tx: tx.clone() };
    let repos = match orchestrator
        .discover(provider.as_ref(), &discovery_progress)
        .await
    {
        Ok(r) => r,
        Err(e) => {
            let _ = tx.send(AppEvent::Backend(BackendMessage::DiscoveryError(format!(
                "Discovery failed: {}",
                e
            ))));
            return;
        }
    };

    // Send discovery results to populate org browser
    let _ = tx.send(AppEvent::Backend(BackendMessage::DiscoveryComplete(
        repos.clone(),
    )));

    if repos.is_empty() {
        let _ = tx.send(AppEvent::Backend(BackendMessage::OperationComplete(
            OpSummary::new(),
        )));
        return;
    }

    // Ensure base path exists
    if !base_path.exists() {
        if let Err(e) = std::fs::create_dir_all(&base_path) {
            let _ = tx.send(AppEvent::Backend(BackendMessage::OperationError(format!(
                "Failed to create base directory: {}",
                e
            ))));
            return;
        }
    }

    // Plan: which repos to clone (new) and which to sync (existing)
    let git = ShellGit::new();
    let provider_name = provider_entry.kind.to_string().to_lowercase();
    let plan = orchestrator.plan_clone(&base_path, repos.clone(), &provider_name, &git);

    let (to_sync, _skipped) = orchestrator.plan_sync(&base_path, repos, &provider_name, &git, true);

    // Send OperationStarted so the UI transitions to Running state
    let clone_count = plan.to_clone.len();
    let sync_count = to_sync.len();
    let total = clone_count + sync_count;
    let _ = tx.send(AppEvent::Backend(BackendMessage::OperationStarted {
        operation: Operation::Sync,
        total,
        to_clone: clone_count,
        to_sync: sync_count,
    }));

    let concurrency = workspace.concurrency.unwrap_or(config.concurrency);
    let mut combined_summary = OpSummary::new();

    // Phase 1: Clone new repos
    if !plan.to_clone.is_empty() {
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
            .with_concurrency(concurrency)
            .with_clone_options(clone_options)
            .with_structure(structure.clone())
            .with_ssh(provider_entry.prefer_ssh);

        let manager = CloneManager::new(ShellGit::new(), manager_options);
        let progress: Arc<dyn CloneProgress> = Arc::new(TuiCloneProgress { tx: tx.clone() });
        let (clone_summary, _results) = manager
            .clone_repos(&base_path, plan.to_clone, &provider_name, progress)
            .await;
        combined_summary.success += clone_summary.success;
        combined_summary.failed += clone_summary.failed;
        combined_summary.skipped += clone_summary.skipped;
    }

    // Phase 2: Sync existing repos
    let sync_mode = if pull_mode {
        SyncMode::Pull
    } else {
        match workspace.sync_mode.unwrap_or(config.sync_mode) {
            crate::config::SyncMode::Pull => SyncMode::Pull,
            crate::config::SyncMode::Fetch => SyncMode::Fetch,
        }
    };

    if !to_sync.is_empty() {
        let manager_options = SyncManagerOptions::new()
            .with_concurrency(concurrency)
            .with_mode(sync_mode)
            .with_skip_uncommitted(true);

        let manager = SyncManager::new(ShellGit::new(), manager_options);
        let progress: Arc<dyn SyncProgress> = Arc::new(TuiSyncProgress { tx: tx.clone() });
        let (sync_summary, _results) = manager.sync_repos(to_sync, progress).await;
        combined_summary.success += sync_summary.success;
        combined_summary.failed += sync_summary.failed;
        combined_summary.skipped += sync_summary.skipped;
    }

    let _ = tx.send(AppEvent::Backend(BackendMessage::OperationComplete(
        combined_summary,
    )));
}

/// Scans local repositories and gets their git status.
async fn run_status_scan(
    config: Config,
    workspace: Option<WorkspaceConfig>,
    tx: UnboundedSender<AppEvent>,
) {
    let workspace = match workspace {
        Some(ws) => ws,
        None => {
            let _ = tx.send(AppEvent::Backend(BackendMessage::OperationError(
                "No workspace selected.".to_string(),
            )));
            return;
        }
    };

    let base_path = workspace.expanded_base_path();
    if !base_path.exists() {
        let _ = tx.send(AppEvent::Backend(BackendMessage::StatusResults(vec![])));
        return;
    }

    let structure = workspace
        .structure
        .clone()
        .unwrap_or_else(|| config.structure.clone());

    let entries = tokio::task::spawn_blocking(move || {
        let git = ShellGit::new();
        let orchestrator = DiscoveryOrchestrator::new(workspace.filters.clone(), structure);
        let local_repos = orchestrator.scan_local(&base_path, &git);
        let mut entries = Vec::new();

        for (path, org, name) in &local_repos {
            let full_name = format!("{}/{}", org, name);
            match git.status(path) {
                Ok(s) => {
                    entries.push(RepoEntry {
                        owner: org.clone(),
                        name: name.clone(),
                        full_name,
                        path: path.clone(),
                        branch: if s.branch.is_empty() {
                            None
                        } else {
                            Some(s.branch)
                        },
                        is_uncommitted: s.is_uncommitted || s.has_untracked,
                        ahead: s.ahead as usize,
                        behind: s.behind as usize,
                        staged_count: s.staged_count,
                        unstaged_count: s.unstaged_count,
                        untracked_count: s.untracked_count,
                    });
                }
                Err(_) => {
                    entries.push(RepoEntry {
                        owner: org.clone(),
                        name: name.clone(),
                        full_name,
                        path: path.clone(),
                        branch: None,
                        is_uncommitted: false,
                        ahead: 0,
                        behind: 0,
                        staged_count: 0,
                        unstaged_count: 0,
                        untracked_count: 0,
                    });
                }
            }
        }
        entries
    })
    .await
    .unwrap_or_default();

    let _ = tx.send(AppEvent::Backend(BackendMessage::StatusResults(entries)));
}
