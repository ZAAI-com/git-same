//! Parallel cloning operations.
//!
//! This module provides the ability to clone multiple repositories
//! concurrently with controlled parallelism.

use crate::git::{CloneOptions, GitOperations};
use crate::types::{OpResult, OpSummary, OwnedRepo};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Semaphore;

/// Maximum allowed concurrency to prevent resource exhaustion.
/// Higher values can cause "too many open files" errors and network saturation.
pub const MAX_CONCURRENCY: usize = 16;

/// Minimum concurrency (at least one clone at a time).
pub const MIN_CONCURRENCY: usize = 1;

/// Progress callback for clone operations.
pub trait CloneProgress: Send + Sync {
    /// Called when a clone starts.
    fn on_start(&self, repo: &OwnedRepo, index: usize, total: usize);

    /// Called when a clone completes successfully.
    fn on_complete(&self, repo: &OwnedRepo, index: usize, total: usize);

    /// Called when a clone fails.
    fn on_error(&self, repo: &OwnedRepo, error: &str, index: usize, total: usize);

    /// Called when a clone is skipped.
    fn on_skip(&self, repo: &OwnedRepo, reason: &str, index: usize, total: usize);
}

/// A no-op progress implementation for when no progress reporting is needed.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoProgress;

impl CloneProgress for NoProgress {
    fn on_start(&self, _repo: &OwnedRepo, _index: usize, _total: usize) {}
    fn on_complete(&self, _repo: &OwnedRepo, _index: usize, _total: usize) {}
    fn on_error(&self, _repo: &OwnedRepo, _error: &str, _index: usize, _total: usize) {}
    fn on_skip(&self, _repo: &OwnedRepo, _reason: &str, _index: usize, _total: usize) {}
}

/// Result of a single clone operation.
#[derive(Debug)]
pub struct CloneResult {
    /// The repository that was cloned
    pub repo: OwnedRepo,
    /// The local path where it was cloned
    pub path: PathBuf,
    /// The operation result
    pub result: OpResult,
}

/// Options for the clone manager.
#[derive(Debug, Clone)]
pub struct CloneManagerOptions {
    /// Maximum number of concurrent clones
    pub concurrency: usize,
    /// Clone options (depth, branch, submodules)
    pub clone_options: CloneOptions,
    /// Directory structure template
    /// Supports: {provider}, {org}, {repo}
    pub structure: String,
    /// Whether to use SSH URLs (vs HTTPS)
    pub prefer_ssh: bool,
    /// Whether this is a dry run
    pub dry_run: bool,
}

impl Default for CloneManagerOptions {
    fn default() -> Self {
        Self {
            concurrency: 4,
            clone_options: CloneOptions::default(),
            structure: "{org}/{repo}".to_string(),
            prefer_ssh: true,
            dry_run: false,
        }
    }
}

impl CloneManagerOptions {
    /// Creates new options with defaults.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the concurrency level, clamped to [MIN_CONCURRENCY, MAX_CONCURRENCY].
    ///
    /// Returns the options with the effective concurrency set.
    /// Use [`effective_concurrency`] to check if the value was capped.
    pub fn with_concurrency(mut self, concurrency: usize) -> Self {
        self.concurrency = concurrency.clamp(MIN_CONCURRENCY, MAX_CONCURRENCY);
        self
    }

    /// Checks if a requested concurrency exceeds the maximum.
    ///
    /// Returns `Some(MAX_CONCURRENCY)` if capping occurred, `None` otherwise.
    pub fn check_concurrency_cap(requested: usize) -> Option<usize> {
        if requested > MAX_CONCURRENCY {
            Some(MAX_CONCURRENCY)
        } else {
            None
        }
    }

    /// Sets the clone options.
    pub fn with_clone_options(mut self, options: CloneOptions) -> Self {
        self.clone_options = options;
        self
    }

    /// Sets the directory structure.
    pub fn with_structure(mut self, structure: impl Into<String>) -> Self {
        self.structure = structure.into();
        self
    }

    /// Sets SSH preference.
    pub fn with_ssh(mut self, prefer_ssh: bool) -> Self {
        self.prefer_ssh = prefer_ssh;
        self
    }

    /// Sets dry run mode.
    pub fn with_dry_run(mut self, dry_run: bool) -> Self {
        self.dry_run = dry_run;
        self
    }
}

/// Manages parallel clone operations.
pub struct CloneManager<G: GitOperations> {
    git: Arc<G>,
    options: CloneManagerOptions,
}

impl<G: GitOperations + 'static> CloneManager<G> {
    /// Creates a new clone manager.
    pub fn new(git: G, options: CloneManagerOptions) -> Self {
        Self {
            git: Arc::new(git),
            options,
        }
    }

    /// Computes the local path for a repository.
    pub fn compute_path(&self, base_path: &Path, repo: &OwnedRepo, provider: &str) -> PathBuf {
        let path_str = self
            .options
            .structure
            .replace("{provider}", provider)
            .replace("{org}", &repo.owner)
            .replace("{repo}", &repo.repo.name);

        base_path.join(path_str)
    }

    /// Gets the clone URL for a repository.
    pub fn get_clone_url<'a>(&self, repo: &'a OwnedRepo) -> &'a str {
        if self.options.prefer_ssh {
            &repo.repo.ssh_url
        } else {
            &repo.repo.clone_url
        }
    }

    /// Clones repositories in parallel.
    ///
    /// Returns a summary of operations and individual results.
    pub async fn clone_repos(
        &self,
        base_path: &Path,
        repos: Vec<OwnedRepo>,
        provider: &str,
        progress: Arc<dyn CloneProgress>,
    ) -> (OpSummary, Vec<CloneResult>) {
        let total = repos.len();
        let semaphore = Arc::new(Semaphore::new(self.options.concurrency));
        let mut handles = Vec::with_capacity(total);

        for (index, repo) in repos.into_iter().enumerate() {
            let permit = semaphore.clone().acquire_owned().await.unwrap();
            let git = self.git.clone();
            let clone_options = self.options.clone_options.clone();
            let target_path = self.compute_path(base_path, &repo, provider);
            let url = self.get_clone_url(&repo).to_string();
            let dry_run = self.options.dry_run;
            let progress = Arc::clone(&progress);

            let handle = tokio::spawn(async move {
                // Notify progress - clone starting
                progress.on_start(&repo, index, total);
                let result = if dry_run {
                    OpResult::Skipped("dry run".to_string())
                } else if target_path.exists() {
                    OpResult::Skipped("directory already exists".to_string())
                } else {
                    // Create parent directories
                    if let Some(parent) = target_path.parent() {
                        if let Err(e) = std::fs::create_dir_all(parent) {
                            OpResult::Failed(format!("Failed to create directory: {}", e))
                        } else {
                            // Perform the clone (blocking operation)
                            match tokio::task::spawn_blocking({
                                let git = git.clone();
                                let url = url.clone();
                                let target_path = target_path.clone();
                                let clone_options = clone_options.clone();
                                move || git.clone_repo(&url, &target_path, &clone_options)
                            })
                            .await
                            {
                                Ok(Ok(())) => OpResult::Success,
                                Ok(Err(e)) => OpResult::Failed(e.to_string()),
                                Err(e) => OpResult::Failed(format!("Task panicked: {}", e)),
                            }
                        }
                    } else {
                        OpResult::Failed("Invalid target path".to_string())
                    }
                };

                drop(permit); // Release semaphore

                CloneResult {
                    repo,
                    path: target_path,
                    result,
                }
            });

            handles.push(handle);
        }

        // Collect results
        let mut summary = OpSummary::new();
        let mut results = Vec::with_capacity(total);

        for (index, handle) in handles.into_iter().enumerate() {
            match handle.await {
                Ok(clone_result) => {
                    // Notify progress
                    match &clone_result.result {
                        OpResult::Success => {
                            progress.on_complete(&clone_result.repo, index, total);
                        }
                        OpResult::Failed(err) => {
                            progress.on_error(&clone_result.repo, err, index, total);
                        }
                        OpResult::Skipped(reason) => {
                            progress.on_skip(&clone_result.repo, reason, index, total);
                        }
                    }

                    summary.record(&clone_result.result);
                    results.push(clone_result);
                }
                Err(e) => {
                    // Task panicked - create a failed result
                    // Note: We don't have the repo here, so we can't report it properly
                    // This should be rare in practice
                    summary.record(&OpResult::Failed(format!("Task panicked: {}", e)));
                }
            }
        }

        (summary, results)
    }

    /// Clones a single repository synchronously.
    pub fn clone_single(&self, base_path: &Path, repo: &OwnedRepo, provider: &str) -> CloneResult {
        let target_path = self.compute_path(base_path, repo, provider);
        let url = self.get_clone_url(repo);

        let result = if self.options.dry_run {
            OpResult::Skipped("dry run".to_string())
        } else if target_path.exists() {
            OpResult::Skipped("directory already exists".to_string())
        } else {
            // Create parent directories
            if let Some(parent) = target_path.parent() {
                if let Err(e) = std::fs::create_dir_all(parent) {
                    OpResult::Failed(format!("Failed to create directory: {}", e))
                } else {
                    match self
                        .git
                        .clone_repo(url, &target_path, &self.options.clone_options)
                    {
                        Ok(()) => OpResult::Success,
                        Err(e) => OpResult::Failed(e.to_string()),
                    }
                }
            } else {
                OpResult::Failed("Invalid target path".to_string())
            }
        };

        CloneResult {
            repo: repo.clone(),
            path: target_path,
            result,
        }
    }
}

#[cfg(test)]
mod tests {
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
        assert_eq!(options.concurrency, 4);
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
}
