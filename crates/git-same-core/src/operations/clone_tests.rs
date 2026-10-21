use super::*;
use crate::git::MockGit;
use crate::types::Repo;
use std::sync::atomic::{AtomicUsize, Ordering};
use tempfile::TempDir;

fn test_repo(name: &str, owner: &str) -> OwnedRepo {
    OwnedRepo::new(owner, Repo::test(name, owner))
}

#[test]
fn test_clone_manager_options_default() {
    let options = CloneManagerOptions::default();
    assert_eq!(options.concurrency, 8);
    assert!(options.prefer_ssh);
    assert!(!options.dry_run);
    assert_eq!(options.structure, "{org}/{repo}");
}

#[test]
fn test_clone_manager_options_builder() {
    let clone_opts = CloneOptions::new().with_depth(1);
    let options = CloneManagerOptions::new()
        .with_concurrency(8)
        .with_clone_options(clone_opts)
        .with_structure("{provider}/{org}/{repo}")
        .with_ssh(false)
        .with_dry_run(true);

    assert_eq!(options.concurrency, 8);
    assert_eq!(options.clone_options.depth, 1);
    assert_eq!(options.structure, "{provider}/{org}/{repo}");
    assert!(!options.prefer_ssh);
    assert!(options.dry_run);
}

#[test]
fn test_concurrency_minimum() {
    let options = CloneManagerOptions::new().with_concurrency(0);
    assert_eq!(options.concurrency, MIN_CONCURRENCY); // Minimum is 1
}

#[test]
fn test_concurrency_maximum() {
    let options = CloneManagerOptions::new().with_concurrency(100);
    assert_eq!(options.concurrency, MAX_CONCURRENCY); // Capped at max
}

#[test]
fn test_concurrency_within_bounds() {
    let options = CloneManagerOptions::new().with_concurrency(8);
    assert_eq!(options.concurrency, 8); // Within bounds, unchanged
}

#[test]
fn test_check_concurrency_cap() {
    assert_eq!(CloneManagerOptions::check_concurrency_cap(8), None);
    assert_eq!(CloneManagerOptions::check_concurrency_cap(16), None);
    assert_eq!(
        CloneManagerOptions::check_concurrency_cap(17),
        Some(MAX_CONCURRENCY)
    );
    assert_eq!(
        CloneManagerOptions::check_concurrency_cap(100),
        Some(MAX_CONCURRENCY)
    );
}

#[test]
fn test_compute_path_simple() {
    let git = MockGit::new();
    let options = CloneManagerOptions::new().with_structure("{org}/{repo}");
    let manager = CloneManager::new(git, options);

    let repo = test_repo("my-repo", "my-org");
    let path = manager.compute_path(Path::new("/base"), &repo, "github");

    assert_eq!(path, PathBuf::from("/base/my-org/my-repo"));
}

#[test]
fn test_compute_path_with_provider() {
    let git = MockGit::new();
    let options = CloneManagerOptions::new().with_structure("{provider}/{org}/{repo}");
    let manager = CloneManager::new(git, options);

    let repo = test_repo("my-repo", "my-org");
    let path = manager.compute_path(Path::new("/base"), &repo, "github");

    assert_eq!(path, PathBuf::from("/base/github/my-org/my-repo"));
}

#[test]
fn test_get_clone_url_ssh() {
    let git = MockGit::new();
    let options = CloneManagerOptions::new().with_ssh(true);
    let manager = CloneManager::new(git, options);

    let repo = test_repo("repo", "org");
    let url = manager.get_clone_url(&repo);

    assert!(url.starts_with("git@"));
}

#[test]
fn test_get_clone_url_https() {
    let git = MockGit::new();
    let options = CloneManagerOptions::new().with_ssh(false);
    let manager = CloneManager::new(git, options);

    let repo = test_repo("repo", "org");
    let url = manager.get_clone_url(&repo);

    assert!(url.starts_with("https://"));
}

#[test]
fn test_clone_single_dry_run() {
    let git = MockGit::new();
    let options = CloneManagerOptions::new().with_dry_run(true);
    let manager = CloneManager::new(git, options);

    let repo = test_repo("repo", "org");
    let result = manager.clone_single(Path::new("/tmp/base"), &repo, "github");

    assert!(result.result.is_skipped());
    assert_eq!(result.result.skip_reason(), Some("dry run"));
}

#[test]
fn test_clone_single_existing_dir() {
    let temp = TempDir::new().unwrap();
    let target = temp.path().join("org/repo");
    std::fs::create_dir_all(&target).unwrap();

    let git = MockGit::new();
    let options = CloneManagerOptions::new();
    let manager = CloneManager::new(git, options);

    let repo = test_repo("repo", "org");
    let result = manager.clone_single(temp.path(), &repo, "github");

    assert!(result.result.is_skipped());
    assert_eq!(
        result.result.skip_reason(),
        Some("directory already exists")
    );
}

#[test]
fn test_clone_single_success() {
    let temp = TempDir::new().unwrap();

    let git = MockGit::new();
    let options = CloneManagerOptions::new();
    let manager = CloneManager::new(git, options);

    let repo = test_repo("repo", "org");
    let result = manager.clone_single(temp.path(), &repo, "github");

    assert!(result.result.is_success());
    assert_eq!(result.path, temp.path().join("org/repo"));
}

#[test]
fn test_clone_single_failure() {
    let temp = TempDir::new().unwrap();

    let mut git = MockGit::new();
    git.fail_clones(Some("network error".to_string()));

    let options = CloneManagerOptions::new();
    let manager = CloneManager::new(git, options);

    let repo = test_repo("repo", "org");
    let result = manager.clone_single(temp.path(), &repo, "github");

    assert!(result.result.is_failed());
    assert!(result
        .result
        .error_message()
        .unwrap()
        .contains("network error"));
}

struct CountingProgress {
    started: AtomicUsize,
    completed: AtomicUsize,
    errors: AtomicUsize,
    skipped: AtomicUsize,
}

impl CountingProgress {
    fn new() -> Self {
        Self {
            started: AtomicUsize::new(0),
            completed: AtomicUsize::new(0),
            errors: AtomicUsize::new(0),
            skipped: AtomicUsize::new(0),
        }
    }
}

impl CloneProgress for CountingProgress {
    fn on_start(&self, _repo: &OwnedRepo, _index: usize, _total: usize) {
        self.started.fetch_add(1, Ordering::SeqCst);
    }

    fn on_complete(&self, _repo: &OwnedRepo, _index: usize, _total: usize) {
        self.completed.fetch_add(1, Ordering::SeqCst);
    }

    fn on_error(&self, _repo: &OwnedRepo, _error: &str, _index: usize, _total: usize) {
        self.errors.fetch_add(1, Ordering::SeqCst);
    }

    fn on_skip(&self, _repo: &OwnedRepo, _reason: &str, _index: usize, _total: usize) {
        self.skipped.fetch_add(1, Ordering::SeqCst);
    }
}

#[tokio::test]
async fn test_clone_repos_parallel() {
    let temp = TempDir::new().unwrap();

    let git = MockGit::new();
    let options = CloneManagerOptions::new().with_concurrency(2);
    let manager = CloneManager::new(git, options);

    let repos = vec![
        test_repo("repo1", "org"),
        test_repo("repo2", "org"),
        test_repo("repo3", "org"),
    ];

    let progress = Arc::new(CountingProgress::new());
    let progress_dyn: Arc<dyn CloneProgress> = progress.clone();
    let (summary, results) = manager
        .clone_repos(temp.path(), repos, "github", progress_dyn)
        .await;

    assert_eq!(summary.success, 3);
    assert_eq!(summary.failed, 0);
    assert_eq!(results.len(), 3);

    // Check progress was called
    assert_eq!(progress.started.load(Ordering::SeqCst), 3);
    assert_eq!(progress.completed.load(Ordering::SeqCst), 3);
}

#[tokio::test]
async fn test_clone_repos_dry_run() {
    let temp = TempDir::new().unwrap();

    let git = MockGit::new();
    let options = CloneManagerOptions::new().with_dry_run(true);
    let manager = CloneManager::new(git, options);

    let repos = vec![test_repo("repo1", "org"), test_repo("repo2", "org")];

    let progress: Arc<dyn CloneProgress> = Arc::new(NoProgress);
    let (summary, _results) = manager
        .clone_repos(temp.path(), repos, "github", progress)
        .await;

    assert_eq!(summary.success, 0);
    assert_eq!(summary.skipped, 2);
}

#[tokio::test]
async fn test_clone_repos_with_failure() {
    let temp = TempDir::new().unwrap();

    let mut git = MockGit::new();
    git.fail_clones(Some("test error".to_string()));

    let options = CloneManagerOptions::new();
    let manager = CloneManager::new(git, options);

    let repos = vec![test_repo("repo1", "org")];

    let progress = Arc::new(CountingProgress::new());
    let progress_dyn: Arc<dyn CloneProgress> = progress.clone();
    let (summary, _results) = manager
        .clone_repos(temp.path(), repos, "github", progress_dyn)
        .await;

    assert_eq!(summary.failed, 1);
    assert_eq!(progress.errors.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn test_clone_repos_zero_concurrency_is_clamped() {
    let temp = TempDir::new().unwrap();

    let git = MockGit::new();
    let mut options = CloneManagerOptions::new().with_dry_run(true);
    options.concurrency = 0; // bypass builder clamp on purpose
    let manager = CloneManager::new(git, options);

    let repos = vec![test_repo("repo1", "org")];
    let progress: Arc<dyn CloneProgress> = Arc::new(NoProgress);
    let (summary, _results) = manager
        .clone_repos(temp.path(), repos, "github", progress)
        .await;

    assert_eq!(summary.skipped, 1);
}

/// Records `start:<repo>:<index>` and `done:<repo>:<index>` in call order.
/// Optionally holds one repository's start for a while to make it finish last.
#[derive(Default)]
struct RecordingProgress {
    events: std::sync::Mutex<Vec<String>>,
    slow: Option<&'static str>,
}

impl RecordingProgress {
    fn record(&self, kind: &str, repo: &OwnedRepo, index: usize) {
        self.events
            .lock()
            .unwrap()
            .push(format!("{kind}:{}:{index}", repo.full_name()));
    }

    fn events(&self) -> Vec<String> {
        self.events.lock().unwrap().clone()
    }
}

impl CloneProgress for RecordingProgress {
    fn on_start(&self, repo: &OwnedRepo, index: usize, _total: usize) {
        self.record("start", repo, index);
        if self.slow == Some(repo.full_name()) {
            std::thread::sleep(std::time::Duration::from_millis(300));
        }
    }

    fn on_complete(&self, repo: &OwnedRepo, index: usize, _total: usize) {
        self.record("done", repo, index);
    }

    fn on_error(&self, repo: &OwnedRepo, _error: &str, index: usize, _total: usize) {
        self.record("done", repo, index);
    }

    fn on_skip(&self, repo: &OwnedRepo, _reason: &str, index: usize, _total: usize) {
        self.record("done", repo, index);
    }
}

#[tokio::test]
async fn test_clone_repos_reports_each_clone_as_it_finishes() {
    let temp = TempDir::new().unwrap();
    let manager = CloneManager::new(
        MockGit::new(),
        CloneManagerOptions::new().with_concurrency(1),
    );
    let repos = vec![
        test_repo("repo1", "org"),
        test_repo("repo2", "org"),
        test_repo("repo3", "org"),
    ];

    let progress = Arc::new(RecordingProgress::default());
    let progress_dyn: Arc<dyn CloneProgress> = progress.clone();
    manager
        .clone_repos(temp.path(), repos, "github", progress_dyn)
        .await;

    // Before, every start was reported before the first finish.
    assert_eq!(
        progress.events(),
        vec![
            "start:org/repo1:0",
            "done:org/repo1:0",
            "start:org/repo2:1",
            "done:org/repo2:1",
            "start:org/repo3:2",
            "done:org/repo3:2",
        ]
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_clone_repos_a_slow_repo_does_not_hold_the_count_back() {
    let temp = TempDir::new().unwrap();
    let options = CloneManagerOptions::new()
        .with_concurrency(3)
        .with_dry_run(true);
    let manager = CloneManager::new(MockGit::new(), options);
    let repos = vec![
        test_repo("repo1", "org"),
        test_repo("repo2", "org"),
        test_repo("repo3", "org"),
    ];

    let progress = Arc::new(RecordingProgress {
        slow: Some("org/repo1"),
        ..Default::default()
    });
    let progress_dyn: Arc<dyn CloneProgress> = progress.clone();
    let (_summary, results) = manager
        .clone_repos(temp.path(), repos, "github", progress_dyn)
        .await;

    let done: Vec<String> = progress
        .events()
        .into_iter()
        .filter(|e| e.starts_with("done:"))
        .collect();
    // The slow first repo finishes last, and the finished callbacks count
    // completions (0, 1, 2) instead of repeating input positions.
    assert_eq!(done.len(), 3);
    assert_eq!(done[2], "done:org/repo1:2");
    assert!(
        done[0].ends_with(":0") && done[1].ends_with(":1"),
        "{done:?}"
    );
    // Results still come back in input order.
    let names: Vec<&str> = results.iter().map(|r| r.repo.full_name()).collect();
    assert_eq!(names, vec!["org/repo1", "org/repo2", "org/repo3"]);
}
