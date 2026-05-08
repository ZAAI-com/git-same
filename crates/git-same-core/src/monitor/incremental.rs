//! Per-repo rescans that mutate an in-memory `FinderStatus` in place.
//!
//! Used by both the FSEvents-driven scan loop and the socket `REFRESH <path>`
//! handler so they share one consistent merge implementation and never need
//! a full `scan_all`.

use crate::api::RepoScanService;
use crate::types::FinderStatus;
use std::path::{Path, PathBuf};

/// Rescan a single repo and merge the result into `status` in place.
///
/// Returns `true` if `status.repos` actually changed.
///
/// If the path no longer looks like a git repo, the corresponding entry is
/// removed.
pub fn rescan_and_merge(
    service: &RepoScanService<'_>,
    status: &mut FinderStatus,
    repo_path: &Path,
) -> bool {
    let canonical = canonicalize_or_via_parent(repo_path);

    if !is_git_repo(&canonical) {
        let before = status.repos.len();
        status.repos.retain(|r| r.path != canonical);
        let removed = status.repos.len() != before;
        if removed {
            status.timestamp = chrono::Utc::now().to_rfc3339();
        }
        return removed;
    }

    let (workspace, org) = labels_for(status, &canonical);
    let new_entry = service.scan_repo(&canonical, workspace.as_deref(), org.as_deref());

    if let Some(existing) = status.repos.iter_mut().find(|r| r.path == canonical) {
        if *existing == new_entry {
            return false;
        }
        *existing = new_entry;
    } else {
        status.repos.push(new_entry);
    }
    status.timestamp = chrono::Utc::now().to_rfc3339();
    true
}

fn is_git_repo(path: &Path) -> bool {
    let dot_git = path.join(".git");
    dot_git.is_dir() || dot_git.is_file() || path.join("HEAD").is_file()
}

/// Recover the workspace + org labels for a repo path.
///
/// Prefers the existing `FinderStatus` entry (so a repo that was first
/// emitted via `scan_all` keeps the labels chosen there). Falls back to
/// matching against `status.workspaces` for newly-discovered paths.
fn labels_for(status: &FinderStatus, repo_path: &Path) -> (Option<String>, Option<String>) {
    if let Some(existing) = status.repos.iter().find(|r| r.path == repo_path) {
        return (existing.workspace.clone(), existing.org.clone());
    }
    for ws in &status.workspaces {
        let ws_root = canonical_or_self(&ws.root);
        if let Ok(rel) = repo_path.strip_prefix(&ws_root) {
            let org = rel
                .components()
                .next()
                .and_then(|c| c.as_os_str().to_str())
                .map(str::to_string);
            return (Some(ws.name.clone()), org);
        }
    }
    (None, None)
}

fn canonical_or_self(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

/// Canonicalize a path, falling back to canonicalizing the parent and
/// rejoining the filename when the path itself no longer exists. Without
/// this fallback, entries stored under the canonical form become orphaned
/// when the repo is deleted on disk and `canonicalize` starts failing.
fn canonicalize_or_via_parent(path: &Path) -> PathBuf {
    if let Ok(canonical) = std::fs::canonicalize(path) {
        return canonical;
    }
    if let (Some(parent), Some(filename)) = (path.parent(), path.file_name()) {
        if let Ok(canonical_parent) = std::fs::canonicalize(parent) {
            return canonical_parent.join(filename);
        }
    }
    path.to_path_buf()
}

#[cfg(test)]
#[path = "incremental_tests.rs"]
mod tests;
