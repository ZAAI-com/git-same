//! Git operations module.
//!
//! This module provides abstractions and implementations for git operations.
//!
//! # Architecture
//!
//! The module is built around the [`GitOperations`] trait, which abstracts
//! git commands like clone, fetch, pull, and status. This allows for:
//!
//! - Real implementations using shell commands ([`ShellGit`])
//! - Mock implementations for testing
//!
//! # Example
//!
//! ```no_run
//! use git_same_core::git::{ShellGit, GitOperations, CloneOptions};
//! use std::path::Path;
//!
//! let git = ShellGit::new();
//!
//! // Clone a repository
//! let options = CloneOptions::new().with_depth(1);
//! git.clone_repo(
//!     "git@github.com:user/repo.git",
//!     Path::new("/tmp/repo"),
//!     &options
//! ).expect("Clone failed");
//!
//! // Check status
//! let status = git.status(Path::new("/tmp/repo")).expect("Status failed");
//! if status.is_clean_and_synced() {
//!     println!("Repository is clean and in sync");
//! }
//! ```

pub mod shell;
pub mod traits;

pub use shell::ShellGit;
pub use traits::{CloneOptions, FetchResult, GitOperations, PullResult, RepoStatus};

#[cfg(test)]
pub use traits::mock::{MockConfig, MockGit};

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
