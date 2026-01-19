//! Clone operations module.
//!
//! This module provides functionality for cloning repositories,
//! including parallel cloning with controlled concurrency.
//!
//! # Example
//!
//! ```no_run
//! use git_same::clone::{CloneManager, CloneManagerOptions, NoProgress};
//! use git_same::git::ShellGit;
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
//!     .clone_repos(Path::new("~/github"), repos, "github", &progress)
//!     .await;
//!
//! println!("Cloned {} repos, {} failed", summary.success, summary.failed);
//! # }
//! ```

pub mod parallel;

pub use parallel::{CloneManager, CloneManagerOptions, CloneProgress, CloneResult, NoProgress};
