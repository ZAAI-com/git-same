//! Backend integration — bridges TUI with existing async command handlers.
//!
//! Provides channel-based progress adapters and spawn functions for operations.

use std::path::Path;
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;

use crate::auth::get_auth;
use crate::config::Config;
use crate::discovery::DiscoveryOrchestrator;
use crate::git::{FetchResult, PullResult, ShellGit};
use crate::operations::clone::{CloneManager, CloneManagerOptions, CloneProgress};
use crate::operations::sync::{SyncManager, SyncManagerOptions, SyncMode, SyncProgress};
use crate::provider::{create_provider, DiscoveryProgress};
use crate::types::{OpSummary, OwnedRepo};

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
        let _ = self
            .tx
            .send(AppEvent::Backend(BackendMessage::RepoProgress {
                repo_name: repo.full_name().to_string(),
                success: true,
                message: "cloning...".to_string(),
            }));
    }

    fn on_complete(&self, repo: &OwnedRepo, _index: usize, _total: usize) {
        let _ = self
            .tx
            .send(AppEvent::Backend(BackendMessage::RepoProgress {
                repo_name: repo.full_name().to_string(),
                success: true,
                message: "cloned".to_string(),
            }));
    }

    fn on_error(&self, repo: &OwnedRepo, error: &str, _index: usize, _total: usize) {
        let _ = self
            .tx
            .send(AppEvent::Backend(BackendMessage::RepoProgress {
                repo_name: repo.full_name().to_string(),
                success: false,
                message: error.to_string(),
            }));
    }

    fn on_skip(&self, repo: &OwnedRepo, reason: &str, _index: usize, _total: usize) {
        let _ = self
            .tx
            .send(AppEvent::Backend(BackendMessage::RepoProgress {
                repo_name: repo.full_name().to_string(),
                success: true,
                message: format!("skipped: {}", reason),
            }));
    }
}

struct TuiSyncProgress {
    tx: UnboundedSender<AppEvent>,
}

impl SyncProgress for TuiSyncProgress {
    fn on_start(&self, repo: &OwnedRepo, _path: &Path, _index: usize, _total: usize) {
        let _ = self
            .tx
            .send(AppEvent::Backend(BackendMessage::RepoProgress {
                repo_name: repo.full_name().to_string(),
                success: true,
                message: "syncing...".to_string(),
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
                message: status.to_string(),
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
                message: status.to_string(),
            }));
    }

    fn on_error(&self, repo: &OwnedRepo, error: &str, _index: usize, _total: usize) {
        let _ = self
            .tx
            .send(AppEvent::Backend(BackendMessage::RepoProgress {
                repo_name: repo.full_name().to_string(),
                success: false,
                message: error.to_string(),
            }));
    }

    fn on_skip(&self, repo: &OwnedRepo, reason: &str, _index: usize, _total: usize) {
        let _ = self
            .tx
            .send(AppEvent::Backend(BackendMessage::RepoProgress {
                repo_name: repo.full_name().to_string(),
                success: true,
                message: format!("skipped: {}", reason),
            }));
    }
}

// -- Spawn functions --

/// Spawn a backend operation as a Tokio task.
pub fn spawn_operation(operation: Operation, app: &App, tx: UnboundedSender<AppEvent>) {
    let config = app.config.clone();
    let base_path = app.base_path.clone();

    match operation {
        Operation::Clone => {
            tokio::spawn(async move {
                run_clone_operation(config, base_path, tx).await;
            });
        }
        Operation::Fetch => {
            tokio::spawn(async move {
                run_sync_operation(config, base_path, tx, SyncMode::Fetch).await;
            });
        }
        Operation::Pull => {
            tokio::spawn(async move {
                run_sync_operation(config, base_path, tx, SyncMode::Pull).await;
            });
        }
        Operation::Status => {
            let repos = app.local_repos.clone();
            tokio::spawn(async move {
                // Status is just re-scanning local repos — handled by the caller
                // For now, send empty results to clear the loading state
                let _ = tx.send(AppEvent::Backend(BackendMessage::StatusResults(repos)));
            });
        }
    }
}

async fn run_clone_operation(
    config: Config,
    base_path: Option<std::path::PathBuf>,
    tx: UnboundedSender<AppEvent>,
) {
    let base_path = match base_path {
        Some(p) => p,
        None => {
            let _ = tx.send(AppEvent::Backend(BackendMessage::OperationError(
                "No base path configured. Set base_path in your provider config.".to_string(),
            )));
            return;
        }
    };

    // Authenticate
    let auth = match get_auth(None) {
        Ok(a) => a,
        Err(e) => {
            let _ = tx.send(AppEvent::Backend(BackendMessage::OperationError(format!(
                "Auth failed: {}",
                e
            ))));
            return;
        }
    };

    // Get provider
    let provider_entry = match config.enabled_providers().next() {
        Some(p) => p.clone(),
        None => {
            let _ = tx.send(AppEvent::Backend(BackendMessage::OperationError(
                "No enabled providers configured".to_string(),
            )));
            return;
        }
    };

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

    // Discover
    let orchestrator = DiscoveryOrchestrator::new(config.filters.clone(), config.structure.clone());
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

    // Plan clone
    let git = ShellGit::new();
    let plan = orchestrator.plan_clone(&base_path, repos, "github", &git);

    if plan.to_clone.is_empty() {
        let _ = tx.send(AppEvent::Backend(BackendMessage::OperationComplete(
            OpSummary::new(),
        )));
        return;
    }

    // Update operation state to Running
    // (The handler will set this when it receives RepoProgress events)

    // Create dirs if needed
    if !base_path.exists() {
        if let Err(e) = std::fs::create_dir_all(&base_path) {
            let _ = tx.send(AppEvent::Backend(BackendMessage::OperationError(format!(
                "Failed to create base directory: {}",
                e
            ))));
            return;
        }
    }

    let clone_options = crate::git::CloneOptions {
        depth: config.clone.depth,
        branch: if config.clone.branch.is_empty() {
            None
        } else {
            Some(config.clone.branch.clone())
        },
        recurse_submodules: config.clone.recurse_submodules,
    };

    let manager_options = CloneManagerOptions::new()
        .with_concurrency(config.concurrency)
        .with_clone_options(clone_options)
        .with_structure(config.structure.clone())
        .with_ssh(true);

    let manager = CloneManager::new(git, manager_options);
    let progress: Arc<dyn CloneProgress> = Arc::new(TuiCloneProgress { tx: tx.clone() });
    let (summary, _results) = manager
        .clone_repos(&base_path, plan.to_clone, "github", progress)
        .await;

    let _ = tx.send(AppEvent::Backend(BackendMessage::OperationComplete(
        summary,
    )));
}

async fn run_sync_operation(
    config: Config,
    base_path: Option<std::path::PathBuf>,
    tx: UnboundedSender<AppEvent>,
    mode: SyncMode,
) {
    let base_path = match base_path {
        Some(p) => p,
        None => {
            let _ = tx.send(AppEvent::Backend(BackendMessage::OperationError(
                "No base path configured. Set base_path in your provider config.".to_string(),
            )));
            return;
        }
    };

    if !base_path.exists() {
        let _ = tx.send(AppEvent::Backend(BackendMessage::OperationError(format!(
            "Base path does not exist: {}",
            base_path.display()
        ))));
        return;
    }

    // Authenticate
    let auth = match get_auth(None) {
        Ok(a) => a,
        Err(e) => {
            let _ = tx.send(AppEvent::Backend(BackendMessage::OperationError(format!(
                "Auth failed: {}",
                e
            ))));
            return;
        }
    };

    // Get provider
    let provider_entry = match config.enabled_providers().next() {
        Some(p) => p.clone(),
        None => {
            let _ = tx.send(AppEvent::Backend(BackendMessage::OperationError(
                "No enabled providers configured".to_string(),
            )));
            return;
        }
    };

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

    // Discover
    let orchestrator = DiscoveryOrchestrator::new(config.filters.clone(), config.structure.clone());
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

    let _ = tx.send(AppEvent::Backend(BackendMessage::DiscoveryComplete(
        repos.clone(),
    )));

    if repos.is_empty() {
        let _ = tx.send(AppEvent::Backend(BackendMessage::OperationComplete(
            OpSummary::new(),
        )));
        return;
    }

    // Plan sync
    let git = ShellGit::new();
    let (to_sync, _skipped) = orchestrator.plan_sync(&base_path, repos, "github", &git, true);

    if to_sync.is_empty() {
        let _ = tx.send(AppEvent::Backend(BackendMessage::OperationComplete(
            OpSummary::new(),
        )));
        return;
    }

    let manager_options = SyncManagerOptions::new()
        .with_concurrency(config.concurrency)
        .with_mode(mode)
        .with_skip_dirty(true);

    let manager = SyncManager::new(git, manager_options);
    let progress: Arc<dyn SyncProgress> = Arc::new(TuiSyncProgress { tx: tx.clone() });
    let (summary, _results) = manager.sync_repos(to_sync, progress).await;

    let _ = tx.send(AppEvent::Backend(BackendMessage::OperationComplete(
        summary,
    )));
}
