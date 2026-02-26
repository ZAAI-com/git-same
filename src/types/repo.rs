//! Repository and organization type definitions.
//!
//! These types represent the data structures returned by Git hosting provider APIs
//! and used internally for clone/sync planning.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A GitHub/GitLab/Bitbucket organization.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Org {
    /// Organization login/username (e.g., "rust-lang")
    pub login: String,
    /// Unique ID from the provider
    pub id: u64,
    /// Optional description
    #[serde(default)]
    pub description: Option<String>,
}

impl Org {
    /// Creates a new organization with just login and id.
    pub fn new(login: impl Into<String>, id: u64) -> Self {
        Self {
            login: login.into(),
            id,
            description: None,
        }
    }
}

/// A repository from a Git hosting provider.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Repo {
    /// Unique ID from the provider
    pub id: u64,
    /// Repository name (e.g., "gisa")
    pub name: String,
    /// Full name including owner (e.g., "user/gisa")
    pub full_name: String,
    /// SSH clone URL (e.g., "git@github.com:user/gisa.git")
    pub ssh_url: String,
    /// HTTPS clone URL (e.g., "https://github.com/user/gisa.git")
    pub clone_url: String,
    /// Default branch name (e.g., "main")
    pub default_branch: String,
    /// Whether this is a private repository
    #[serde(default)]
    pub private: bool,
    /// Whether this repository is archived (read-only)
    #[serde(default)]
    pub archived: bool,
    /// Whether this is a fork of another repository
    #[serde(default)]
    pub fork: bool,
    /// When the repository was last pushed to
    #[serde(default)]
    pub pushed_at: Option<DateTime<Utc>>,
    /// Optional description
    #[serde(default)]
    pub description: Option<String>,
}

impl Repo {
    /// Creates a minimal repo for testing.
    #[cfg(test)]
    pub fn test(name: &str, owner: &str) -> Self {
        Self {
            id: rand_id(),
            name: name.to_string(),
            full_name: format!("{}/{}", owner, name),
            ssh_url: format!("git@github.com:{}/{}.git", owner, name),
            clone_url: format!("https://github.com/{}/{}.git", owner, name),
            default_branch: "main".to_string(),
            private: false,
            archived: false,
            fork: false,
            pushed_at: None,
            description: None,
        }
    }

    /// Returns the owner from the full_name.
    pub fn owner(&self) -> &str {
        self.full_name.split('/').next().unwrap_or(&self.full_name)
    }
}

#[cfg(test)]
fn rand_id() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(1);
    COUNTER.fetch_add(1, Ordering::Relaxed)
}

/// A repository with its owner information.
///
/// This type pairs a repository with the owner that it was discovered under,
/// which may be an organization or the user's personal account.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OwnedRepo {
    /// Organization name or username
    pub owner: String,
    /// The repository
    pub repo: Repo,
}

impl OwnedRepo {
    /// Creates a new owned repo.
    pub fn new(owner: impl Into<String>, repo: Repo) -> Self {
        Self {
            owner: owner.into(),
            repo,
        }
    }

    /// Returns the full path for this repo (e.g., "org/repo").
    pub fn full_name(&self) -> &str {
        &self.repo.full_name
    }

    /// Returns the repository name.
    pub fn name(&self) -> &str {
        &self.repo.name
    }
}

/// Result of comparing discovered repos with local filesystem.
///
/// This represents the action plan for a clone/sync operation.
#[derive(Debug, Default)]
pub struct ActionPlan {
    /// New repositories that need to be cloned
    pub to_clone: Vec<OwnedRepo>,
    /// Existing repositories that should be synced
    pub to_sync: Vec<OwnedRepo>,
    /// Repositories that were skipped (already exist, uncommitted changes, etc.)
    pub skipped: Vec<SkippedRepo>,
}

impl ActionPlan {
    /// Creates an empty action plan.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the total number of repositories in the plan.
    pub fn total(&self) -> usize {
        self.to_clone.len() + self.to_sync.len() + self.skipped.len()
    }

    /// Returns true if there's nothing to do.
    pub fn is_empty(&self) -> bool {
        self.to_clone.is_empty() && self.to_sync.is_empty()
    }

    /// Adds a repo to clone.
    pub fn add_clone(&mut self, repo: OwnedRepo) {
        self.to_clone.push(repo);
    }

    /// Adds a repo to sync.
    pub fn add_sync(&mut self, repo: OwnedRepo) {
        self.to_sync.push(repo);
    }

    /// Adds a skipped repo.
    pub fn add_skipped(&mut self, repo: OwnedRepo, reason: impl Into<String>) {
        self.skipped.push(SkippedRepo {
            repo,
            reason: reason.into(),
        });
    }
}

/// A repository that was skipped during planning.
#[derive(Debug)]
pub struct SkippedRepo {
    /// The repository that was skipped
    pub repo: OwnedRepo,
    /// Reason for skipping
    pub reason: String,
}

/// Outcome of a single clone or sync operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpResult {
    /// Operation completed successfully
    Success,
    /// Operation failed with an error
    Failed(String),
    /// Operation was skipped for a reason
    Skipped(String),
}

impl OpResult {
    /// Returns true if the operation was successful.
    pub fn is_success(&self) -> bool {
        matches!(self, OpResult::Success)
    }

    /// Returns true if the operation failed.
    pub fn is_failed(&self) -> bool {
        matches!(self, OpResult::Failed(_))
    }

    /// Returns true if the operation was skipped.
    pub fn is_skipped(&self) -> bool {
        matches!(self, OpResult::Skipped(_))
    }

    /// Returns the error message if failed.
    pub fn error_message(&self) -> Option<&str> {
        match self {
            OpResult::Failed(msg) => Some(msg),
            _ => None,
        }
    }

    /// Returns the skip reason if skipped.
    pub fn skip_reason(&self) -> Option<&str> {
        match self {
            OpResult::Skipped(reason) => Some(reason),
            _ => None,
        }
    }
}

/// Summary statistics for a batch operation.
#[derive(Debug, Default, Clone)]
pub struct OpSummary {
    /// Number of successful operations
    pub success: usize,
    /// Number of failed operations
    pub failed: usize,
    /// Number of skipped operations
    pub skipped: usize,
}

impl OpSummary {
    /// Creates an empty summary.
    pub fn new() -> Self {
        Self::default()
    }

    /// Records a result.
    pub fn record(&mut self, result: &OpResult) {
        match result {
            OpResult::Success => self.success += 1,
            OpResult::Failed(_) => self.failed += 1,
            OpResult::Skipped(_) => self.skipped += 1,
        }
    }

    /// Returns the total number of operations.
    pub fn total(&self) -> usize {
        self.success + self.failed + self.skipped
    }

    /// Returns true if there were any failures.
    pub fn has_failures(&self) -> bool {
        self.failed > 0
    }
}

#[cfg(test)]
#[path = "repo_tests.rs"]
mod tests;
