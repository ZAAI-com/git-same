//! Backend integration — bridges TUI with existing async command handlers.
//!
//! Provides channel-based progress adapters and spawn functions for operations.

use std::path::Path;
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;

use crate::config::{Config, WorkspaceConfig, WorkspaceProvider};
use crate::git::{FetchResult, GitOperations, PullResult, ShellGit};
use crate::operations::clone::CloneProgress;
use crate::operations::sync::SyncProgress;
use crate::provider::DiscoveryProgress;
use crate::types::{OpSummary, OwnedRepo};
use crate::workflows::status_scan::scan_workspace_status;
use crate::workflows::sync_workspace::{
    execute_prepared_sync, prepare_sync_workspace, SyncWorkspaceRequest,
};

use super::app::{App, Operation};
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

/// Spawn setup-wizard org discovery without blocking the TUI event loop.
pub fn spawn_setup_org_discovery(
    ws_provider: WorkspaceProvider,
    token: String,
    tx: UnboundedSender<AppEvent>,
) {
    tokio::spawn(async move {
        match crate::setup::handler::discover_org_entries(ws_provider, token).await {
            Ok(orgs) => {
                let _ = tx.send(AppEvent::Backend(BackendMessage::SetupOrgsDiscovered(orgs)));
            }
            Err(err) => {
                let _ = tx.send(AppEvent::Backend(BackendMessage::SetupOrgsError(err)));
            }
        }
    });
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

    let discovery_progress = TuiDiscoveryProgress { tx: tx.clone() };
    let prepared = match prepare_sync_workspace(
        SyncWorkspaceRequest {
            config: &config,
            workspace: &workspace,
            refresh: true,
            skip_uncommitted: true,
            pull: pull_mode,
            concurrency_override: None,
            create_base_path: true,
        },
        &discovery_progress,
    )
    .await
    {
        Ok(p) => p,
        Err(e) => {
            let _ = tx.send(AppEvent::Backend(BackendMessage::OperationError(format!(
                "{}",
                e
            ))));
            return;
        }
    };

    // Send discovery results to populate org browser
    let _ = tx.send(AppEvent::Backend(BackendMessage::DiscoveryComplete(
        prepared.repos.clone(),
    )));

    if prepared.repos.is_empty() {
        let _ = tx.send(AppEvent::Backend(BackendMessage::OperationComplete(
            OpSummary::new(),
        )));
        return;
    }

    // Send OperationStarted so the UI transitions to Running state
    let clone_count = prepared.plan.to_clone.len();
    let sync_count = prepared.to_sync.len();
    let total = clone_count + sync_count;
    let _ = tx.send(AppEvent::Backend(BackendMessage::OperationStarted {
        operation: Operation::Sync,
        total,
        to_clone: clone_count,
        to_sync: sync_count,
    }));

    let clone_progress: Arc<dyn CloneProgress> = Arc::new(TuiCloneProgress { tx: tx.clone() });
    let sync_progress: Arc<dyn SyncProgress> = Arc::new(TuiSyncProgress { tx: tx.clone() });
    let outcome = execute_prepared_sync(&prepared, false, clone_progress, sync_progress).await;

    let mut combined_summary = OpSummary::new();
    if let Some(summary) = outcome.clone_summary {
        combined_summary.success += summary.success;
        combined_summary.failed += summary.failed;
        combined_summary.skipped += summary.skipped;
    }
    if let Some(summary) = outcome.sync_summary {
        combined_summary.success += summary.success;
        combined_summary.failed += summary.failed;
        combined_summary.skipped += summary.skipped;
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

    let entries = tokio::task::spawn_blocking(move || scan_workspace_status(&config, &workspace))
        .await
        .unwrap_or_default();

    let _ = tx.send(AppEvent::Backend(BackendMessage::StatusResults(entries)));
}

#[cfg(test)]
#[path = "backend_tests.rs"]
mod tests;
