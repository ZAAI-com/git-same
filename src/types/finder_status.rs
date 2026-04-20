//! Types for the Finder extension status data.
//!
//! These types define the JSON schema written by the daemon and read by
//! the FinderSync extension. They represent the complete state needed to
//! render badges, icons, and context menus.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Badge color indicating repository health.
///
/// Priority order: Red > Orange > Blue > Green.
/// `Gray` is reserved for ambient (non-workspace) repos that haven't been
/// classified yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Badge {
    /// Everything synced, no local-only data, no important ignored files.
    /// Safe to delete.
    Green,
    /// Fully synced, but has important gitignored files (.env, keys, etc.).
    /// Code is on GitHub, but local secrets/config would be lost.
    Blue,
    /// Main branch clean & synced, but other branches or worktrees diverge.
    /// Main branch is safe; other branches or worktrees have local-only data.
    Orange,
    /// Staged, unstaged, untracked, or unpushed commits.
    /// DO NOT delete — uncommitted work or unpushed commits would be lost.
    Red,
    /// Ambient git repo discovered outside any configured workspace.
    /// Upgraded to a semantic color on demand (right-click → REFRESH /path).
    Gray,
}

/// Branch sync status in the context menu.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FinderBranchInfo {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub upstream: Option<String>,
    pub ahead: u32,
    pub behind: u32,
    pub synced: bool,
}

/// Remote info for the context menu.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FinderRemoteInfo {
    pub name: String,
    pub url: String,
}

/// Worktree info for the context menu.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FinderWorktreeInfo {
    pub path: PathBuf,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    pub synced: bool,
}

/// Complete status for a single repository.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FinderRepoStatus {
    pub path: PathBuf,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub org: Option<String>,
    pub badge: Badge,
    pub current_branch: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_branch: Option<String>,
    pub commit_count: u64,
    pub staged_count: usize,
    pub unstaged_count: usize,
    pub untracked_count: usize,
    pub ahead: u32,
    pub behind: u32,
    pub stash_count: usize,
    pub has_important_ignored_files: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub important_ignored_files: Vec<String>,
    pub branches: Vec<FinderBranchInfo>,
    pub all_branches_synced: bool,
    pub remotes: Vec<FinderRemoteInfo>,
    pub worktrees: Vec<FinderWorktreeInfo>,
    pub all_worktrees_synced: bool,
}

/// Classification of a GitHub account that owns repositories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum OwnerType {
    /// Personal GitHub account.
    User,
    /// GitHub Organization account.
    Organization,
    /// Not yet classified (cache miss) or classification failed.
    #[default]
    Unknown,
}

/// An organization or user folder inside a git-same workspace.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrgFolderInfo {
    pub path: PathBuf,
    pub org: String,
    pub workspace: String,
    #[serde(default)]
    pub owner_type: OwnerType,
}

/// Workspace summary for the status file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FinderWorkspaceInfo {
    pub name: String,
    pub root: PathBuf,
    pub orgs: Vec<String>,
}

/// Top-level status file written by the daemon.
///
/// This is the single source of truth read by the FinderSync extension.
/// Written atomically to `~/.config/git-same/finder/status.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FinderStatus {
    pub version: u32,
    pub timestamp: String,
    pub daemon_pid: u32,
    pub workspaces: Vec<FinderWorkspaceInfo>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub custom_folders: Vec<PathBuf>,
    pub repos: Vec<FinderRepoStatus>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub org_folders: Vec<OrgFolderInfo>,
    /// Union of workspace roots and ambient scan roots. The FinderSync
    /// extension registers these as `FIFinderSyncController.directoryURLs`
    /// so Finder knows which folders to ask about.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub monitored_roots: Vec<PathBuf>,
}

impl FinderStatus {
    /// Current schema version.
    pub const VERSION: u32 = 1;

    /// Creates a new empty status.
    pub fn new(pid: u32, timestamp: String) -> Self {
        Self {
            version: Self::VERSION,
            timestamp,
            daemon_pid: pid,
            workspaces: Vec::new(),
            custom_folders: Vec::new(),
            repos: Vec::new(),
            org_folders: Vec::new(),
            monitored_roots: Vec::new(),
        }
    }
}

/// Default patterns for detecting important gitignored files.
///
/// These patterns indicate files that contain secrets, credentials, or
/// local configuration that would be lost if the repository were deleted.
pub const DEFAULT_IMPORTANT_IGNORED_PATTERNS: &[&str] = &[
    ".env",
    ".env.*",
    "*.key",
    "*.pem",
    "*.p12",
    "*.pfx",
    "credentials*",
    "secrets*",
    ".secret*",
    "service-account*.json",
    "*.keystore",
];

/// Compute the badge color for a repository based on its status.
pub fn compute_badge(
    staged: usize,
    unstaged: usize,
    untracked: usize,
    ahead: u32,
    all_branches_synced: bool,
    all_worktrees_synced: bool,
    has_important_ignored_files: bool,
) -> Badge {
    // Red: any local-only changes or unpushed commits
    if staged > 0 || unstaged > 0 || untracked > 0 || ahead > 0 {
        return Badge::Red;
    }

    // Orange: main branch clean, but other branches/worktrees not synced
    if !all_branches_synced || !all_worktrees_synced {
        return Badge::Orange;
    }

    // Blue: everything synced but has important ignored files
    if has_important_ignored_files {
        return Badge::Blue;
    }

    // Green: fully clean and synced
    Badge::Green
}

/// Check whether a file path matches any of the important ignored patterns.
///
/// Uses simple glob-like matching: `*` matches any characters, `?` matches one.
pub fn matches_important_pattern(file_path: &str, patterns: &[&str]) -> bool {
    let filename = std::path::Path::new(file_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(file_path);

    for pattern in patterns {
        if simple_glob_match(pattern, filename) {
            return true;
        }
    }
    false
}

/// Simple glob matching supporting `*` (any chars) and `?` (single char).
fn simple_glob_match(pattern: &str, text: &str) -> bool {
    glob_match_recursive(
        &pattern.chars().collect::<Vec<_>>(),
        0,
        &text.chars().collect::<Vec<_>>(),
        0,
    )
}

fn glob_match_recursive(pattern: &[char], pi: usize, text: &[char], ti: usize) -> bool {
    if pi == pattern.len() && ti == text.len() {
        return true;
    }
    if pi == pattern.len() {
        return false;
    }

    match pattern[pi] {
        '*' => {
            // Try matching * with 0, 1, 2, ... characters
            for i in ti..=text.len() {
                if glob_match_recursive(pattern, pi + 1, text, i) {
                    return true;
                }
            }
            false
        }
        '?' => {
            if ti < text.len() {
                glob_match_recursive(pattern, pi + 1, text, ti + 1)
            } else {
                false
            }
        }
        c => {
            if ti < text.len() && text[ti] == c {
                glob_match_recursive(pattern, pi + 1, text, ti + 1)
            } else {
                false
            }
        }
    }
}

#[cfg(test)]
#[path = "finder_status_tests.rs"]
mod tests;
