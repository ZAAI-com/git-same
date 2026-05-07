//! Domain types describing a local repository's git state.
//!
//! Lifted out of the TUI module so non-UI callers (`workflows::status_scan`,
//! `cache::sync_history`) can use them without depending on `tui::*`.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// A summary entry for sync history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncHistoryEntry {
    pub timestamp: String,
    pub duration_secs: f64,
    pub success: usize,
    pub failed: usize,
    pub skipped: usize,
    pub with_updates: usize,
    pub cloned: usize,
    pub total_new_commits: u32,
}

/// A local repo with its computed status.
#[derive(Debug, Clone)]
pub struct RepoEntry {
    pub owner: String,
    pub name: String,
    pub full_name: String,
    pub path: PathBuf,
    pub branch: Option<String>,
    pub is_uncommitted: bool,
    pub ahead: usize,
    pub behind: usize,
    pub staged_count: usize,
    pub unstaged_count: usize,
    pub untracked_count: usize,
}
