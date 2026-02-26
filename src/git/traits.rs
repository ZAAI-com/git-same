//! Git operations trait definitions.
//!
//! This module defines the trait abstractions for git operations,
//! allowing for both real and mock implementations for testing.

use crate::errors::GitError;
use std::path::Path;

/// Options for cloning a repository.
#[derive(Debug, Clone, Default)]
pub struct CloneOptions {
    /// Clone depth (0 = full clone)
    pub depth: u32,
    /// Specific branch to clone
    pub branch: Option<String>,
    /// Whether to recurse into submodules
    pub recurse_submodules: bool,
}

impl CloneOptions {
    /// Creates new clone options with defaults.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the clone depth.
    pub fn with_depth(mut self, depth: u32) -> Self {
        self.depth = depth;
        self
    }

    /// Sets the branch to clone.
    pub fn with_branch(mut self, branch: impl Into<String>) -> Self {
        self.branch = Some(branch.into());
        self
    }

    /// Enables recursive submodule cloning.
    pub fn with_submodules(mut self) -> Self {
        self.recurse_submodules = true;
        self
    }
}

/// Status of a local repository.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoStatus {
    /// Current branch name
    pub branch: String,
    /// Whether the working tree has uncommitted changes
    pub is_uncommitted: bool,
    /// Number of commits ahead of upstream
    pub ahead: u32,
    /// Number of commits behind upstream
    pub behind: u32,
    /// Whether there are untracked files
    pub has_untracked: bool,
    /// Number of staged (index) changes
    pub staged_count: usize,
    /// Number of unstaged (working tree) changes
    pub unstaged_count: usize,
    /// Number of untracked files
    pub untracked_count: usize,
}

impl RepoStatus {
    /// Returns true if the repo is clean and in sync with upstream.
    pub fn is_clean_and_synced(&self) -> bool {
        !self.is_uncommitted && !self.has_untracked && self.ahead == 0 && self.behind == 0
    }

    /// Returns true if it's safe to do a fast-forward pull.
    pub fn can_fast_forward(&self) -> bool {
        !self.is_uncommitted && self.ahead == 0 && self.behind > 0
    }
}

/// Result of a fetch operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FetchResult {
    /// Whether any new commits were fetched
    pub updated: bool,
    /// Number of new commits (if available)
    pub new_commits: Option<u32>,
}

/// Result of a pull operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PullResult {
    /// Whether the pull was successful
    pub success: bool,
    /// Whether this was a fast-forward
    pub fast_forward: bool,
    /// Error message if not successful
    pub error: Option<String>,
}

/// Trait for git operations.
///
/// This trait abstracts git commands to allow for testing with mocks.
pub trait GitOperations: Send + Sync {
    /// Clones a repository to the target path.
    ///
    /// # Arguments
    /// * `url` - The clone URL (SSH or HTTPS)
    /// * `target` - Target directory path
    /// * `options` - Clone options (depth, branch, submodules)
    fn clone_repo(&self, url: &str, target: &Path, options: &CloneOptions) -> Result<(), GitError>;

    /// Fetches updates from the remote.
    ///
    /// # Arguments
    /// * `repo_path` - Path to the local repository
    fn fetch(&self, repo_path: &Path) -> Result<FetchResult, GitError>;

    /// Pulls updates from the remote.
    ///
    /// # Arguments
    /// * `repo_path` - Path to the local repository
    fn pull(&self, repo_path: &Path) -> Result<PullResult, GitError>;

    /// Gets the status of a local repository.
    ///
    /// # Arguments
    /// * `repo_path` - Path to the local repository
    fn status(&self, repo_path: &Path) -> Result<RepoStatus, GitError>;

    /// Checks if a directory is a git repository.
    ///
    /// # Arguments
    /// * `path` - Path to check
    fn is_repo(&self, path: &Path) -> bool;

    /// Gets the current branch name.
    ///
    /// # Arguments
    /// * `repo_path` - Path to the local repository
    fn current_branch(&self, repo_path: &Path) -> Result<String, GitError>;

    /// Gets the remote URL for a repository.
    ///
    /// # Arguments
    /// * `repo_path` - Path to the local repository
    /// * `remote` - Remote name (default: "origin")
    fn remote_url(&self, repo_path: &Path, remote: &str) -> Result<String, GitError>;

    /// Gets recent commits as one-line summaries.
    ///
    /// # Arguments
    /// * `repo_path` - Path to the local repository
    /// * `limit` - Maximum number of commits to return
    fn recent_commits(&self, repo_path: &Path, limit: usize) -> Result<Vec<String>, GitError>;
}

/// A mock implementation of GitOperations for testing.
#[cfg(test)]
pub mod mock {
    use super::*;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    /// Records of operations performed.
    #[derive(Debug, Clone, Default)]
    pub struct MockCallLog {
        pub clones: Vec<(String, String, CloneOptions)>, // (url, path, options)
        pub fetches: Vec<String>,                        // paths
        pub pulls: Vec<String>,                          // paths
        pub status_checks: Vec<String>,                  // paths
    }

    /// Configuration for mock responses.
    #[derive(Debug, Clone)]
    pub struct MockConfig {
        /// Whether clone operations should succeed
        pub clone_succeeds: bool,
        /// Whether fetch operations should succeed
        pub fetch_succeeds: bool,
        /// Whether pull operations should succeed
        pub pull_succeeds: bool,
        /// Whether fetch reports updates
        pub fetch_has_updates: bool,
        /// Default status to return
        pub default_status: RepoStatus,
        /// Custom statuses per path
        pub path_statuses: HashMap<String, RepoStatus>,
        /// Paths that are valid repos
        pub valid_repos: Vec<String>,
        /// Custom error message for failures
        pub error_message: Option<String>,
    }

    impl Default for MockConfig {
        fn default() -> Self {
            Self {
                clone_succeeds: true,
                fetch_succeeds: true,
                pull_succeeds: true,
                fetch_has_updates: false,
                default_status: RepoStatus {
                    branch: "main".to_string(),
                    is_uncommitted: false,
                    ahead: 0,
                    behind: 0,
                    has_untracked: false,
                    staged_count: 0,
                    unstaged_count: 0,
                    untracked_count: 0,
                },
                path_statuses: HashMap::new(),
                valid_repos: Vec::new(),
                error_message: None,
            }
        }
    }

    /// Mock git operations for testing.
    pub struct MockGit {
        config: MockConfig,
        log: Arc<Mutex<MockCallLog>>,
    }

    impl MockGit {
        /// Creates a new mock with default configuration.
        pub fn new() -> Self {
            Self {
                config: MockConfig::default(),
                log: Arc::new(Mutex::new(MockCallLog::default())),
            }
        }

        /// Creates a new mock with custom configuration.
        pub fn with_config(config: MockConfig) -> Self {
            Self {
                config,
                log: Arc::new(Mutex::new(MockCallLog::default())),
            }
        }

        /// Gets the call log.
        pub fn call_log(&self) -> MockCallLog {
            self.log.lock().unwrap().clone()
        }

        /// Marks a path as a valid repo.
        pub fn add_repo(&mut self, path: impl Into<String>) {
            self.config.valid_repos.push(path.into());
        }

        /// Sets a custom status for a path.
        pub fn set_status(&mut self, path: impl Into<String>, status: RepoStatus) {
            self.config.path_statuses.insert(path.into(), status);
        }

        /// Configures clone to fail.
        pub fn fail_clones(&mut self, message: Option<String>) {
            self.config.clone_succeeds = false;
            self.config.error_message = message;
        }

        /// Configures fetch to fail.
        pub fn fail_fetches(&mut self, message: Option<String>) {
            self.config.fetch_succeeds = false;
            self.config.error_message = message;
        }

        /// Configures pull to fail.
        pub fn fail_pulls(&mut self, message: Option<String>) {
            self.config.pull_succeeds = false;
            self.config.error_message = message;
        }
    }

    impl Default for MockGit {
        fn default() -> Self {
            Self::new()
        }
    }

    impl GitOperations for MockGit {
        fn clone_repo(
            &self,
            url: &str,
            target: &Path,
            options: &CloneOptions,
        ) -> Result<(), GitError> {
            let mut log = self.log.lock().unwrap();
            log.clones.push((
                url.to_string(),
                target.to_string_lossy().to_string(),
                options.clone(),
            ));

            if self.config.clone_succeeds {
                Ok(())
            } else {
                Err(GitError::clone_failed(
                    url,
                    self.config
                        .error_message
                        .as_deref()
                        .unwrap_or("mock clone failure"),
                ))
            }
        }

        fn fetch(&self, repo_path: &Path) -> Result<FetchResult, GitError> {
            let mut log = self.log.lock().unwrap();
            log.fetches.push(repo_path.to_string_lossy().to_string());

            if self.config.fetch_succeeds {
                Ok(FetchResult {
                    updated: self.config.fetch_has_updates,
                    new_commits: if self.config.fetch_has_updates {
                        Some(3)
                    } else {
                        Some(0)
                    },
                })
            } else {
                Err(GitError::fetch_failed(
                    repo_path,
                    self.config
                        .error_message
                        .as_deref()
                        .unwrap_or("mock fetch failure"),
                ))
            }
        }

        fn pull(&self, repo_path: &Path) -> Result<PullResult, GitError> {
            let mut log = self.log.lock().unwrap();
            log.pulls.push(repo_path.to_string_lossy().to_string());

            if self.config.pull_succeeds {
                Ok(PullResult {
                    success: true,
                    fast_forward: true,
                    error: None,
                })
            } else {
                Err(GitError::pull_failed(
                    repo_path,
                    self.config
                        .error_message
                        .as_deref()
                        .unwrap_or("mock pull failure"),
                ))
            }
        }

        fn status(&self, repo_path: &Path) -> Result<RepoStatus, GitError> {
            let mut log = self.log.lock().unwrap();
            let path_str = repo_path.to_string_lossy().to_string();
            log.status_checks.push(path_str.clone());

            if let Some(status) = self.config.path_statuses.get(&path_str) {
                Ok(status.clone())
            } else {
                Ok(self.config.default_status.clone())
            }
        }

        fn is_repo(&self, path: &Path) -> bool {
            let path_str = path.to_string_lossy().to_string();
            self.config.valid_repos.contains(&path_str)
        }

        fn current_branch(&self, repo_path: &Path) -> Result<String, GitError> {
            let path_str = repo_path.to_string_lossy().to_string();
            if let Some(status) = self.config.path_statuses.get(&path_str) {
                Ok(status.branch.clone())
            } else {
                Ok(self.config.default_status.branch.clone())
            }
        }

        fn remote_url(&self, _repo_path: &Path, _remote: &str) -> Result<String, GitError> {
            Ok("git@github.com:example/repo.git".to_string())
        }

        fn recent_commits(
            &self,
            _repo_path: &Path,
            _limit: usize,
        ) -> Result<Vec<String>, GitError> {
            Ok(Vec::new())
        }
    }
}

#[cfg(test)]
#[path = "traits_tests.rs"]
mod tests;
