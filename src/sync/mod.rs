//! Sync operations module.
//!
//! This module provides functionality for syncing existing local repositories
//! with their remotes, including parallel fetch and pull operations.
//!
//! # Example
//!
//! ```no_run
//! use git_same::sync::{SyncManager, SyncManagerOptions, SyncMode, LocalRepo, NoSyncProgress};
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

pub mod manager;

pub use manager::{
    LocalRepo, NoSyncProgress, SyncManager, SyncManagerOptions, SyncMode, SyncProgress, SyncResult,
};
