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
    assert!(options.skip_uncommitted);
    assert!(!options.dry_run);
}

#[test]
fn test_sync_manager_options_builder() {
    let options = SyncManagerOptions::new()
        .with_concurrency(8)
        .with_mode(SyncMode::Pull)
        .with_skip_uncommitted(false)
        .with_dry_run(true);

    assert_eq!(options.concurrency, 8);
    assert_eq!(options.mode, SyncMode::Pull);
    assert!(!options.skip_uncommitted);
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
fn test_sync_single_uncommitted_skip() {
    let temp = TempDir::new().unwrap();

    let mut git = MockGit::new();
    let path_str = temp.path().to_string_lossy().to_string();
    git.add_repo(path_str.clone());
    git.set_status(
        path_str,
        RepoStatus {
            branch: "main".to_string(),
            is_uncommitted: true,
            ahead: 0,
            behind: 0,
            has_untracked: false,
            staged_count: 0,
            unstaged_count: 0,
            untracked_count: 0,
        },
    );

    let options = SyncManagerOptions::new().with_skip_uncommitted(true);
    let manager = SyncManager::new(git, options);

    let repo = local_repo("repo", "org", temp.path());
    let result = manager.sync_single(&repo);

    assert!(result.result.is_skipped());
    assert_eq!(result.result.skip_reason(), Some("uncommitted changes"));
}

#[test]
fn test_sync_single_fetch_success() {
    let temp = TempDir::new().unwrap();

    let mut git = MockGit::new();
    git.add_repo(temp.path().to_string_lossy().to_string());
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
    let mut git = MockGit::with_config(config);
    git.add_repo(temp.path().to_string_lossy().to_string());

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
    git.add_repo(temp.path().to_string_lossy().to_string());
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

    let mut git = MockGit::new();
    git.add_repo(temp1.path().to_string_lossy().to_string());
    git.add_repo(temp2.path().to_string_lossy().to_string());
    git.add_repo(temp3.path().to_string_lossy().to_string());
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

    let mut git = MockGit::new();
    git.add_repo(temp.path().to_string_lossy().to_string());
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
    let mut git = MockGit::with_config(config);
    git.add_repo(temp.path().to_string_lossy().to_string());

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

#[tokio::test]
async fn test_sync_repos_zero_concurrency_is_clamped() {
    let temp = TempDir::new().unwrap();

    let mut git = MockGit::new();
    git.add_repo(temp.path().to_string_lossy().to_string());
    let mut options = SyncManagerOptions::new().with_dry_run(true);
    options.concurrency = 0; // bypass builder clamp on purpose
    let manager = SyncManager::new(git, options);

    let repos = vec![local_repo("repo", "org", temp.path())];
    let progress: Arc<dyn SyncProgress> = Arc::new(NoSyncProgress);
    let (summary, _results) = manager.sync_repos(repos, progress).await;

    assert_eq!(summary.skipped, 1);
}

/// Panics when a sync task starts for `repo`, after counting the call.
struct PanicOnStart {
    repo: String,
    counts: CountingSyncProgress,
}

impl SyncProgress for PanicOnStart {
    fn on_start(&self, repo: &OwnedRepo, path: &Path, index: usize, total: usize) {
        self.counts.on_start(repo, path, index, total);
        if repo.full_name() == self.repo {
            panic!("progress reporter blew up");
        }
    }

    fn on_fetch_complete(&self, repo: &OwnedRepo, result: &FetchResult, i: usize, t: usize) {
        self.counts.on_fetch_complete(repo, result, i, t);
    }

    fn on_pull_complete(&self, repo: &OwnedRepo, result: &PullResult, i: usize, t: usize) {
        self.counts.on_pull_complete(repo, result, i, t);
    }

    fn on_error(&self, repo: &OwnedRepo, error: &str, index: usize, total: usize) {
        self.counts.on_error(repo, error, index, total);
    }

    fn on_skip(&self, repo: &OwnedRepo, reason: &str, index: usize, total: usize) {
        self.counts.on_skip(repo, reason, index, total);
    }
}

#[tokio::test]
async fn test_sync_repos_panicked_task_is_reported_and_recorded() {
    let temp1 = TempDir::new().unwrap();
    let temp2 = TempDir::new().unwrap();

    let mut git = MockGit::new();
    git.add_repo(temp1.path().to_string_lossy().to_string());
    git.add_repo(temp2.path().to_string_lossy().to_string());
    let manager = SyncManager::new(git, SyncManagerOptions::new().with_concurrency(2));

    let repos = vec![
        local_repo("repo1", "org", temp1.path()),
        local_repo("repo2", "org", temp2.path()),
    ];
    let progress = Arc::new(PanicOnStart {
        repo: "org/repo2".to_string(),
        counts: CountingSyncProgress::new(),
    });
    let progress_dyn: Arc<dyn SyncProgress> = progress.clone();
    let (summary, results) = manager.sync_repos(repos, progress_dyn).await;

    assert_eq!(summary.success, 1);
    assert_eq!(summary.failed, 1);
    assert_eq!(results.len(), 2, "the panicked repo has a result too");
    let failed = results.iter().find(|r| r.result.is_failed()).unwrap();
    assert_eq!(failed.repo.full_name(), "org/repo2");
    assert!(failed.result.error_message().unwrap().contains("panicked"));
    assert_eq!(progress.counts.errors.load(Ordering::SeqCst), 1);
}
