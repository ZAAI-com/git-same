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
    pub skip_dirty: bool,
    /// Whether this is a dry run
    pub dry_run: bool,
}

impl Default for SyncManagerOptions {
    fn default() -> Self {
        Self {
            concurrency: crate::operations::clone::DEFAULT_CONCURRENCY,
            mode: SyncMode::Fetch,
            skip_dirty: true,
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

    /// Sets whether to skip dirty repos.
    pub fn with_skip_dirty(mut self, skip_dirty: bool) -> Self {
        self.skip_dirty = skip_dirty;
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
    pub fn new(git: G, options: SyncManagerOptions) -> Self {
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
        let semaphore = Arc::new(Semaphore::new(self.options.concurrency));
        let mut handles = Vec::with_capacity(total);

        for (index, local_repo) in repos.into_iter().enumerate() {
            let permit = semaphore.clone().acquire_owned().await.unwrap();
            let git = self.git.clone();
            let mode = self.options.mode;
            let skip_dirty = self.options.skip_dirty;
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
                let status = tokio::task::spawn_blocking({
                    let git = git.clone();
                    let path = path.clone();
                    move || git.status(&path)
                })
                .await
                .ok()
                .and_then(|r| r.ok());

                // Check if dirty and should skip
                if skip_dirty {
                    if let Some(ref s) = status {
                        if s.is_dirty || s.has_untracked {
                            drop(permit);
                            return SyncResult {
                                repo: local_repo.repo,
                                path,
                                result: OpResult::Skipped("working tree is dirty".to_string()),
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
        let status = self.git.status(path).ok();

        // Check if dirty
        if self.options.skip_dirty {
            if let Some(ref s) = status {
                if s.is_dirty || s.has_untracked {
                    return SyncResult {
                        repo: local_repo.repo.clone(),
                        path: path.clone(),
                        result: OpResult::Skipped("working tree is dirty".to_string()),
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
mod tests {
    use super::*;
    use crate::git::{MockConfig, MockGit, RepoStatus};
    use crate::types::Repo;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tempfile::TempDir;

    fn test_repo(name: &str, owner: &str) -> OwnedRepo {
        OwnedRepo::new(owner, Repo::test(name, owner))
    }

    fn local_repo(name: &str, owner: &str, path: impl Into<PathBuf>) -> LocalRepo {
        LocalRepo::new(test_repo(name, owner), path)
    }

    #[test]
    fn test_sync_manager_options_default() {
        let options = SyncManagerOptions::default();
        assert_eq!(options.concurrency, 8);
        assert_eq!(options.mode, SyncMode::Fetch);
        assert!(options.skip_dirty);
        assert!(!options.dry_run);
    }

    #[test]
    fn test_sync_manager_options_builder() {
        let options = SyncManagerOptions::new()
            .with_concurrency(8)
            .with_mode(SyncMode::Pull)
            .with_skip_dirty(false)
            .with_dry_run(true);

        assert_eq!(options.concurrency, 8);
        assert_eq!(options.mode, SyncMode::Pull);
        assert!(!options.skip_dirty);
        assert!(options.dry_run);
    }

    #[test]
    fn test_sync_single_path_not_exists() {
        let git = MockGit::new();
        let options = SyncManagerOptions::new();
        let manager = SyncManager::new(git, options);

        let repo = local_repo("repo", "org", "/nonexistent/path");
        let result = manager.sync_single(&repo);

        assert!(result.result.is_skipped());
        assert_eq!(result.result.skip_reason(), Some("path does not exist"));
    }

    #[test]
    fn test_sync_single_dry_run() {
        let temp = TempDir::new().unwrap();

        let mut git = MockGit::new();
        git.add_repo(temp.path().to_string_lossy().to_string());

        let options = SyncManagerOptions::new().with_dry_run(true);
        let manager = SyncManager::new(git, options);

        let repo = local_repo("repo", "org", temp.path());
        let result = manager.sync_single(&repo);

        assert!(result.result.is_skipped());
        assert_eq!(result.result.skip_reason(), Some("dry run"));
    }

    #[test]
    fn test_sync_single_dirty_skip() {
        let temp = TempDir::new().unwrap();

        let mut git = MockGit::new();
        let path_str = temp.path().to_string_lossy().to_string();
        git.add_repo(path_str.clone());
        git.set_status(
            path_str,
            RepoStatus {
                branch: "main".to_string(),
                is_dirty: true,
                ahead: 0,
                behind: 0,
                has_untracked: false,
            },
        );

        let options = SyncManagerOptions::new().with_skip_dirty(true);
        let manager = SyncManager::new(git, options);

        let repo = local_repo("repo", "org", temp.path());
        let result = manager.sync_single(&repo);

        assert!(result.result.is_skipped());
        assert_eq!(result.result.skip_reason(), Some("working tree is dirty"));
    }

    #[test]
    fn test_sync_single_fetch_success() {
        let temp = TempDir::new().unwrap();

        let git = MockGit::new();
        let options = SyncManagerOptions::new().with_mode(SyncMode::Fetch);
        let manager = SyncManager::new(git, options);

        let repo = local_repo("repo", "org", temp.path());
        let result = manager.sync_single(&repo);

        assert!(result.result.is_success());
    }

    #[test]
    fn test_sync_single_pull_success() {
        let temp = TempDir::new().unwrap();

        let config = MockConfig {
            fetch_has_updates: true,
            ..Default::default()
        };
        let git = MockGit::with_config(config);

        let options = SyncManagerOptions::new().with_mode(SyncMode::Pull);
        let manager = SyncManager::new(git, options);

        let repo = local_repo("repo", "org", temp.path());
        let result = manager.sync_single(&repo);

        assert!(result.result.is_success());
        assert!(result.had_updates);
    }

    #[test]
    fn test_sync_single_fetch_failure() {
        let temp = TempDir::new().unwrap();

        let mut git = MockGit::new();
        git.fail_fetches(Some("network error".to_string()));

        let options = SyncManagerOptions::new();
        let manager = SyncManager::new(git, options);

        let repo = local_repo("repo", "org", temp.path());
        let result = manager.sync_single(&repo);

        assert!(result.result.is_failed());
        assert!(result
            .result
            .error_message()
            .unwrap()
            .contains("network error"));
    }

    struct CountingSyncProgress {
        started: AtomicUsize,
        fetch_complete: AtomicUsize,
        pull_complete: AtomicUsize,
        errors: AtomicUsize,
        skipped: AtomicUsize,
    }

    impl CountingSyncProgress {
        fn new() -> Self {
            Self {
                started: AtomicUsize::new(0),
                fetch_complete: AtomicUsize::new(0),
                pull_complete: AtomicUsize::new(0),
                errors: AtomicUsize::new(0),
                skipped: AtomicUsize::new(0),
            }
        }
    }

    impl SyncProgress for CountingSyncProgress {
        fn on_start(&self, _repo: &OwnedRepo, _path: &Path, _index: usize, _total: usize) {
            self.started.fetch_add(1, Ordering::SeqCst);
        }

        fn on_fetch_complete(
            &self,
            _repo: &OwnedRepo,
            _result: &FetchResult,
            _index: usize,
            _total: usize,
        ) {
            self.fetch_complete.fetch_add(1, Ordering::SeqCst);
        }

        fn on_pull_complete(
            &self,
            _repo: &OwnedRepo,
            _result: &PullResult,
            _index: usize,
            _total: usize,
        ) {
            self.pull_complete.fetch_add(1, Ordering::SeqCst);
        }

        fn on_error(&self, _repo: &OwnedRepo, _error: &str, _index: usize, _total: usize) {
            self.errors.fetch_add(1, Ordering::SeqCst);
        }

        fn on_skip(&self, _repo: &OwnedRepo, _reason: &str, _index: usize, _total: usize) {
            self.skipped.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[tokio::test]
    async fn test_sync_repos_parallel() {
        let temp1 = TempDir::new().unwrap();
        let temp2 = TempDir::new().unwrap();
        let temp3 = TempDir::new().unwrap();

        let git = MockGit::new();
        let options = SyncManagerOptions::new().with_concurrency(2);
        let manager = SyncManager::new(git, options);

        let repos = vec![
            local_repo("repo1", "org", temp1.path()),
            local_repo("repo2", "org", temp2.path()),
            local_repo("repo3", "org", temp3.path()),
        ];

        let progress = Arc::new(CountingSyncProgress::new());
        let progress_dyn: Arc<dyn SyncProgress> = progress.clone();
        let (summary, results) = manager.sync_repos(repos, progress_dyn).await;

        assert_eq!(summary.success, 3);
        assert_eq!(results.len(), 3);
        assert_eq!(progress.started.load(Ordering::SeqCst), 3);
        assert_eq!(progress.fetch_complete.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_sync_repos_dry_run() {
        let temp = TempDir::new().unwrap();

        let git = MockGit::new();
        let options = SyncManagerOptions::new().with_dry_run(true);
        let manager = SyncManager::new(git, options);

        let repos = vec![local_repo("repo", "org", temp.path())];

        let progress: Arc<dyn SyncProgress> = Arc::new(NoSyncProgress);
        let (summary, _results) = manager.sync_repos(repos, progress).await;

        assert_eq!(summary.skipped, 1);
    }

    #[tokio::test]
    async fn test_sync_repos_with_updates_pull_mode() {
        let temp = TempDir::new().unwrap();

        let config = MockConfig {
            fetch_has_updates: true,
            ..Default::default()
        };
        let git = MockGit::with_config(config);

        let options = SyncManagerOptions::new().with_mode(SyncMode::Pull);
        let manager = SyncManager::new(git, options);

        let repos = vec![local_repo("repo", "org", temp.path())];

        let progress = Arc::new(CountingSyncProgress::new());
        let progress_dyn: Arc<dyn SyncProgress> = progress.clone();
        let (summary, results) = manager.sync_repos(repos, progress_dyn).await;

        assert_eq!(summary.success, 1);
        assert!(results[0].had_updates);
        assert_eq!(progress.pull_complete.load(Ordering::SeqCst), 1);
    }
}
