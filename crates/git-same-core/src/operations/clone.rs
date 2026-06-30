//! Parallel cloning operations.
//!
//! This module provides functionality for cloning repositories,
//! including parallel cloning with controlled concurrency.
//!
//! # Example
//!
//! ```no_run
//! use git_same_core::operations::clone::{CloneManager, CloneManagerOptions, NoProgress};
//! use git_same_core::git::ShellGit;
//! use std::path::Path;
//!
//! # async fn example() {
//! let git = ShellGit::new();
//! let options = CloneManagerOptions::new()
//!     .with_concurrency(4)
//!     .with_structure("{org}/{repo}");
//!
//! let manager = CloneManager::new(git, options);
//!
//! // repos would come from discovery
//! let repos = vec![];
//! let progress = NoProgress;
//!
//! let (summary, results) = manager
//!     .clone_repos(Path::new("~/github"), repos, "github", std::sync::Arc::new(progress))
//!     .await;
//!
//! println!("Cloned {} repos, {} failed", summary.success, summary.failed);
//! # }
//! ```

use crate::domain::RepoPathTemplate;
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

/// Default concurrency when not specified in config.
pub const DEFAULT_CONCURRENCY: usize = 8;

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
            concurrency: DEFAULT_CONCURRENCY,
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
    pub fn new(git: G, mut options: CloneManagerOptions) -> Self {
        options.concurrency = options.concurrency.clamp(MIN_CONCURRENCY, MAX_CONCURRENCY);
        Self {
            git: Arc::new(git),
            options,
        }
    }

    /// Computes the local path for a repository.
    pub fn compute_path(&self, base_path: &Path, repo: &OwnedRepo, provider: &str) -> PathBuf {
        RepoPathTemplate::new(self.options.structure.clone())
            .render_owned_repo(base_path, repo, provider)
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
        let concurrency = self
            .options
            .concurrency
            .clamp(MIN_CONCURRENCY, MAX_CONCURRENCY);
        let semaphore = Arc::new(Semaphore::new(concurrency));
        let mut handles = Vec::with_capacity(total);

        for (index, repo) in repos.into_iter().enumerate() {
            let permit = semaphore.clone().acquire_owned().await.unwrap();
            let git = self.git.clone();
            let clone_options = self.options.clone_options.clone();
            let target_path = self.compute_path(base_path, &repo, provider);
            let url = self.get_clone_url(&repo).to_string();
            let dry_run = self.options.dry_run;
            let progress = Arc::clone(&progress);
            let panic_repo = repo.clone();
            let panic_path = target_path.clone();

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

            handles.push((panic_repo, panic_path, handle));
        }

        // Collect results
        let mut summary = OpSummary::new();
        let mut results = Vec::with_capacity(total);

        for (index, (panic_repo, panic_path, handle)) in handles.into_iter().enumerate() {
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
                    let err = format!("Task panicked: {}", e);
                    progress.on_error(&panic_repo, &err, index, total);
                    let failed = CloneResult {
                        repo: panic_repo,
                        path: panic_path,
                        result: OpResult::Failed(err),
                    };
                    summary.record(&failed.result);
                    results.push(failed);
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
#[path = "clone_tests.rs"]
mod tests;
