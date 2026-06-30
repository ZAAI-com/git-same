//! Shared status scan workflow.

use crate::config::{Config, WorkspaceConfig};
use crate::discovery::DiscoveryOrchestrator;
use crate::git::{GitOperations, ShellGit};
use crate::types::RepoEntry;

/// Scan local repositories for git status for a workspace.
pub fn scan_workspace_status(config: &Config, workspace: &WorkspaceConfig) -> Vec<RepoEntry> {
    let base_path = workspace.expanded_base_path();
    if !base_path.exists() {
        return Vec::new();
    }

    let structure = workspace
        .structure
        .clone()
        .unwrap_or_else(|| config.structure.clone());

    let git = ShellGit::new();
    let orchestrator = DiscoveryOrchestrator::new(workspace.filters.clone(), structure);
    let local_repos = orchestrator.scan_local(&base_path, &git);

    let mut entries = Vec::new();
    for (path, org, name) in &local_repos {
        let full_name = format!("{}/{}", org, name);
        match git.status(path) {
            Ok(s) => entries.push(RepoEntry {
                owner: org.clone(),
                name: name.clone(),
                full_name,
                path: path.clone(),
                branch: if s.branch.is_empty() {
                    None
                } else {
                    Some(s.branch)
                },
                is_uncommitted: s.is_uncommitted || s.has_untracked,
                ahead: s.ahead as usize,
                behind: s.behind as usize,
                staged_count: s.staged_count,
                unstaged_count: s.unstaged_count,
                untracked_count: s.untracked_count,
            }),
            Err(_) => entries.push(RepoEntry {
                owner: org.clone(),
                name: name.clone(),
                full_name,
                path: path.clone(),
                branch: None,
                is_uncommitted: false,
                ahead: 0,
                behind: 0,
                staged_count: 0,
                unstaged_count: 0,
                untracked_count: 0,
            }),
        }
    }

    entries
}

#[cfg(test)]
#[path = "status_scan_tests.rs"]
mod tests;
