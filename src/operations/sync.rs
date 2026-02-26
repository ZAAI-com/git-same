//! Sync manager for fetch and pull operations.
//!
//! This module provides functionality for syncing existing local repositories
//! with their remotes, including parallel fetch and pull operations.
//!
//! # Example
//!
//! ```no_run
//! use git_same::operations::sync::{SyncManager, SyncManagerOptions, SyncMode, LocalRepo, NoSyncProgress};
//! use git_same::git::ShellGit;
//! use git_same::types::{OwnedRepo, Repo};
//! use std::path::PathBuf;
//!
//! # async fn example() {
//! let git = ShellGit::new();
//! let options = SyncManagerOptions::new()
//!     .with_concurrency(4)
//!     .with_mode(SyncMode::Fetch);
//!
//! let manager = SyncManager::new(git, options);
//!
//! // repos would come from discovery
//! let repos: Vec<LocalRepo> = vec![];
//! let progress = NoSyncProgress;
//!
//! let (summary, results) = manager
//!     .sync_repos(repos, std::sync::Arc::new(progress))
//!     .await;
//!
//! println!("Synced {} repos, {} had updates", summary.success,
//!     results.iter().filter(|r| r.had_updates).count());
//! # }
//! ```

use crate::git::{FetchResult, GitOperations, PullResult, RepoStatus};
use crate::types::{OpResult, OpSummary, OwnedRepo};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Semaphore;

use super::clone::{MAX_CONCURRENCY, MIN_CONCURRENCY};

/// Progress callback for sync operations.
pub trait SyncProgress: Send + Sync {
    /// Called when a sync operation starts.
    fn on_start(&self, repo: &OwnedRepo, path: &Path, index: usize, total: usize);

    /// Called when a fetch completes.
    fn on_fetch_complete(&self, repo: &OwnedRepo, result: &FetchResult, index: usize, total: usize);

    /// Called when a pull completes.
    fn on_pull_complete(&self, repo: &OwnedRepo, result: &PullResult, index: usize, total: usize);

    /// Called when a sync fails.
    fn on_error(&self, repo: &OwnedRepo, error: &str, index: usize, total: usize);

    /// Called when a sync is skipped.
    fn on_skip(&self, repo: &OwnedRepo, reason: &str, index: usize, total: usize);
}

/// A no-op progress implementation.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoSyncProgress;

impl SyncProgress for NoSyncProgress {
    fn on_start(&self, _repo: &OwnedRepo, _path: &Path, _index: usize, _total: usize) {}
    fn on_fetch_complete(
        &self,
        _repo: &OwnedRepo,
        _result: &FetchResult,
        _index: usize,
        _total: usize,
    ) {
    }
    fn on_pull_complete(
        &self,
        _repo: &OwnedRepo,
        _result: &PullResult,
        _index: usize,
        _total: usize,
    ) {
    }
    fn on_error(&self, _repo: &OwnedRepo, _error: &str, _index: usize, _total: usize) {}
    fn on_skip(&self, _repo: &OwnedRepo, _reason: &str, _index: usize, _total: usize) {}
}

/// Sync mode - fetch only or pull.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SyncMode {
    /// Only fetch, don't modify working tree
    #[default]
    Fetch,
    /// Fetch and pull (fast-forward only)
    Pull,
}

/// Result of a single sync operation.
#[derive(Debug)]
pub struct SyncResult {
    /// The repository that was synced
    pub repo: OwnedRepo,
    /// The local path
    pub path: PathBuf,
    /// The operation result
    pub result: OpResult,
    /// Whether updates were available
    pub had_updates: bool,
    /// Repository status before sync
    pub status: Option<RepoStatus>,
    /// Fetch result (if fetch was performed)
    pub fetch_result: Option<FetchResult>,
    /// Pull result (if pull was performed)
    pub pull_result: Option<PullResult>,
}

/// A repository with its local path for syncing.
#[derive(Debug, Clone)]
pub struct LocalRepo {
    /// The owned repo metadata
    pub repo: OwnedRepo,
    /// Local filesystem path
    pub path: PathBuf,
}

impl LocalRepo {
    /// Creates a new local repo.
    pub fn new(repo: OwnedRepo, path: impl Into<PathBuf>) -> Self {
        Self {
            repo,
            path: path.into(),
        }
    }
}

/// Options for the sync manager.
#[derive(Debug, Clone)]
pub struct SyncManagerOptions {
    /// Maximum number of concurrent syncs
    pub concurrency: usize,
    /// Sync mode (fetch or pull)
    pub mode: SyncMode,
    /// Skip repos with uncommitted changes
    pub skip_uncommitted: bool,
    /// Whether this is a dry run
    pub dry_run: bool,
}

impl Default for SyncManagerOptions {
    fn default() -> Self {
        Self {
            concurrency: crate::operations::clone::DEFAULT_CONCURRENCY,
            mode: SyncMode::Fetch,
            skip_uncommitted: true,
            dry_run: false,
        }
    }
}

impl SyncManagerOptions {
    /// Creates new options with defaults.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the concurrency level, clamped to [MIN_CONCURRENCY, MAX_CONCURRENCY].
    pub fn with_concurrency(mut self, concurrency: usize) -> Self {
        self.concurrency = concurrency.clamp(MIN_CONCURRENCY, MAX_CONCURRENCY);
        self
    }

    /// Sets the sync mode.
    pub fn with_mode(mut self, mode: SyncMode) -> Self {
        self.mode = mode;
        self
    }

    /// Sets whether to skip uncommitted repos.
    pub fn with_skip_uncommitted(mut self, skip_uncommitted: bool) -> Self {
        self.skip_uncommitted = skip_uncommitted;
        self
    }

    /// Sets dry run mode.
    pub fn with_dry_run(mut self, dry_run: bool) -> Self {
        self.dry_run = dry_run;
        self
    }
}

/// Manages parallel sync operations.
pub struct SyncManager<G: GitOperations> {
    git: Arc<G>,
    options: SyncManagerOptions,
}

impl<G: GitOperations + 'static> SyncManager<G> {
    /// Creates a new sync manager.
    pub fn new(git: G, mut options: SyncManagerOptions) -> Self {
        options.concurrency = options.concurrency.clamp(MIN_CONCURRENCY, MAX_CONCURRENCY);
        Self {
            git: Arc::new(git),
            options,
        }
    }

    /// Syncs repositories in parallel.
    pub async fn sync_repos(
        &self,
        repos: Vec<LocalRepo>,
        progress: Arc<dyn SyncProgress>,
    ) -> (OpSummary, Vec<SyncResult>) {
        let total = repos.len();
        let concurrency = self
            .options
            .concurrency
            .clamp(MIN_CONCURRENCY, MAX_CONCURRENCY);
        let semaphore = Arc::new(Semaphore::new(concurrency));
        let mut handles = Vec::with_capacity(total);

        for (index, local_repo) in repos.into_iter().enumerate() {
            let permit = semaphore.clone().acquire_owned().await.unwrap();
            let git = self.git.clone();
            let mode = self.options.mode;
            let skip_uncommitted = self.options.skip_uncommitted;
            let dry_run = self.options.dry_run;
            let progress = Arc::clone(&progress);

            let handle = tokio::spawn(async move {
                // Notify progress - sync starting
                progress.on_start(&local_repo.repo, &local_repo.path, index, total);
                let path = local_repo.path.clone();

                // Check if path exists and is a repo
                if !path.exists() {
                    drop(permit);
                    return SyncResult {
                        repo: local_repo.repo,
                        path,
                        result: OpResult::Skipped("path does not exist".to_string()),
                        had_updates: false,
                        status: None,
                        fetch_result: None,
                        pull_result: None,
                    };
                }

                // Get status (blocking)
                let status = match tokio::task::spawn_blocking({
                    let git = git.clone();
                    let path = path.clone();
                    move || git.status(&path)
                })
                .await
                {
                    Ok(Ok(status)) => Some(status),
                    Ok(Err(e)) if skip_uncommitted => {
                        drop(permit);
                        return SyncResult {
                            repo: local_repo.repo,
                            path,
                            result: OpResult::Skipped(format!("failed to get status: {}", e)),
                            had_updates: false,
                            status: None,
                            fetch_result: None,
                            pull_result: None,
                        };
                    }
                    Ok(Err(_)) => None,
                    Err(e) if skip_uncommitted => {
                        drop(permit);
                        return SyncResult {
                            repo: local_repo.repo,
                            path,
                            result: OpResult::Skipped(format!(
                                "failed to get status: task join error: {}",
                                e
                            )),
                            had_updates: false,
                            status: None,
                            fetch_result: None,
                            pull_result: None,
                        };
                    }
                    Err(_) => None,
                };

                // Check if uncommitted and should skip
                if skip_uncommitted {
                    if let Some(ref s) = status {
                        if s.is_uncommitted || s.has_untracked {
                            drop(permit);
                            return SyncResult {
                                repo: local_repo.repo,
                                path,
                                result: OpResult::Skipped("uncommitted changes".to_string()),
                                had_updates: false,
                                status,
                                fetch_result: None,
                                pull_result: None,
                            };
                        }
                    }
                }

                // Dry run
                if dry_run {
                    drop(permit);
                    return SyncResult {
                        repo: local_repo.repo,
                        path,
                        result: OpResult::Skipped("dry run".to_string()),
                        had_updates: false,
                        status,
                        fetch_result: None,
                        pull_result: None,
                    };
                }

                // Perform fetch (blocking)
                let fetch_result = tokio::task::spawn_blocking({
                    let git = git.clone();
                    let path = path.clone();
                    move || git.fetch(&path)
                })
                .await;

                let fetch_result = match fetch_result {
                    Ok(Ok(r)) => r,
                    Ok(Err(e)) => {
                        drop(permit);
                        return SyncResult {
                            repo: local_repo.repo,
                            path,
                            result: OpResult::Failed(e.to_string()),
                            had_updates: false,
                            status,
                            fetch_result: None,
                            pull_result: None,
                        };
                    }
                    Err(e) => {
                        drop(permit);
                        return SyncResult {
                            repo: local_repo.repo,
                            path,
                            result: OpResult::Failed(format!("Task panicked: {}", e)),
                            had_updates: false,
                            status,
                            fetch_result: None,
                            pull_result: None,
                        };
                    }
                };

                let had_updates = fetch_result.updated;

                // If pull mode and has updates, do pull
                if mode == SyncMode::Pull && had_updates {
                    let pull_task_result = tokio::task::spawn_blocking({
                        let git = git.clone();
                        let path = path.clone();
                        move || git.pull(&path)
                    })
                    .await;

                    let (result, actual_pull_result) = match pull_task_result {
                        Ok(Ok(r)) if r.success => (OpResult::Success, Some(r)),
                        Ok(Ok(r)) => (
                            OpResult::Failed(
                                r.error.clone().unwrap_or_else(|| "Pull failed".to_string()),
                            ),
                            Some(r),
                        ),
                        Ok(Err(e)) => (OpResult::Failed(e.to_string()), None),
                        Err(e) => (OpResult::Failed(format!("Task panicked: {}", e)), None),
                    };

                    drop(permit);
                    SyncResult {
                        repo: local_repo.repo,
                        path,
                        result,
                        had_updates,
                        status,
                        fetch_result: Some(fetch_result),
                        pull_result: actual_pull_result,
                    }
                } else {
                    drop(permit);
                    SyncResult {
                        repo: local_repo.repo,
                        path,
                        result: OpResult::Success,
                        had_updates,
                        status,
                        fetch_result: Some(fetch_result),
                        pull_result: None,
                    }
                }
            });

            handles.push(handle);
        }

        // Collect results
        let mut summary = OpSummary::new();
        let mut results = Vec::with_capacity(total);

        for (index, handle) in handles.into_iter().enumerate() {
            match handle.await {
                Ok(sync_result) => {
                    // Notify progress based on result using actual operation results
                    match &sync_result.result {
                        OpResult::Success => {
                            if let Some(ref pull_result) = sync_result.pull_result {
                                progress.on_pull_complete(
                                    &sync_result.repo,
                                    pull_result,
                                    index,
                                    total,
                                );
                            } else if let Some(ref fetch_result) = sync_result.fetch_result {
                                progress.on_fetch_complete(
                                    &sync_result.repo,
                                    fetch_result,
                                    index,
                                    total,
                                );
                            }
                        }
                        OpResult::Failed(err) => {
                            progress.on_error(&sync_result.repo, err, index, total);
                        }
                        OpResult::Skipped(reason) => {
                            progress.on_skip(&sync_result.repo, reason, index, total);
                        }
                    }

                    summary.record(&sync_result.result);
                    results.push(sync_result);
                }
                Err(e) => {
                    summary.record(&OpResult::Failed(format!("Task panicked: {}", e)));
                }
            }
        }

        (summary, results)
    }

    /// Syncs a single repository synchronously.
    pub fn sync_single(&self, local_repo: &LocalRepo) -> SyncResult {
        let path = &local_repo.path;

        // Check if path exists
        if !path.exists() {
            return SyncResult {
                repo: local_repo.repo.clone(),
                path: path.clone(),
                result: OpResult::Skipped("path does not exist".to_string()),
                had_updates: false,
                status: None,
                fetch_result: None,
                pull_result: None,
            };
        }

        // Get status
        let status = match self.git.status(path) {
            Ok(status) => Some(status),
            Err(e) if self.options.skip_uncommitted => {
                return SyncResult {
                    repo: local_repo.repo.clone(),
                    path: path.clone(),
                    result: OpResult::Skipped(format!("failed to get status: {}", e)),
                    had_updates: false,
                    status: None,
                    fetch_result: None,
                    pull_result: None,
                };
            }
            Err(_) => None,
        };

        // Check if uncommitted
        if self.options.skip_uncommitted {
            if let Some(ref s) = status {
                if s.is_uncommitted || s.has_untracked {
                    return SyncResult {
                        repo: local_repo.repo.clone(),
                        path: path.clone(),
                        result: OpResult::Skipped("uncommitted changes".to_string()),
                        had_updates: false,
                        status,
                        fetch_result: None,
                        pull_result: None,
                    };
                }
            }
        }

        // Dry run
        if self.options.dry_run {
            return SyncResult {
                repo: local_repo.repo.clone(),
                path: path.clone(),
                result: OpResult::Skipped("dry run".to_string()),
                had_updates: false,
                status,
                fetch_result: None,
                pull_result: None,
            };
        }

        // Fetch
        let fetch_result = match self.git.fetch(path) {
            Ok(r) => r,
            Err(e) => {
                return SyncResult {
                    repo: local_repo.repo.clone(),
                    path: path.clone(),
                    result: OpResult::Failed(e.to_string()),
                    had_updates: false,
                    status,
                    fetch_result: None,
                    pull_result: None,
                };
            }
        };

        let had_updates = fetch_result.updated;

        // Pull if needed
        if self.options.mode == SyncMode::Pull && had_updates {
            match self.git.pull(path) {
                Ok(r) if r.success => SyncResult {
                    repo: local_repo.repo.clone(),
                    path: path.clone(),
                    result: OpResult::Success,
                    had_updates,
                    status,
                    fetch_result: Some(fetch_result),
                    pull_result: Some(r),
                },
                Ok(r) => SyncResult {
                    repo: local_repo.repo.clone(),
                    path: path.clone(),
                    result: OpResult::Failed(
                        r.error.clone().unwrap_or_else(|| "Pull failed".to_string()),
                    ),
                    had_updates,
                    status,
                    fetch_result: Some(fetch_result),
                    pull_result: Some(r),
                },
                Err(e) => SyncResult {
                    repo: local_repo.repo.clone(),
                    path: path.clone(),
                    result: OpResult::Failed(e.to_string()),
                    had_updates,
                    status,
                    fetch_result: Some(fetch_result),
                    pull_result: None,
                },
            }
        } else {
            SyncResult {
                repo: local_repo.repo.clone(),
                path: path.clone(),
                result: OpResult::Success,
                had_updates,
                status,
                fetch_result: Some(fetch_result),
                pull_result: None,
            }
        }
    }
}

#[cfg(test)]
#[path = "sync_tests.rs"]
mod tests;
