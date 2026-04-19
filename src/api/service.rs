//! Repository scanning service.
//!
//! `RepoScanService` is the API for scanning repositories and computing badge
//! status. It owns no state — callers construct it with references to a git
//! backend and a config, then invoke `scan_all()`, `scan_workspace()`, or
//! `scan_repo()`.

use crate::config::{Config, WorkspaceConfig, WorkspaceStore};
use crate::discovery::DiscoveryOrchestrator;
use crate::errors::Result;
use crate::git::GitOperations;
use crate::types::finder_status::{
    compute_badge, matches_important_pattern, FinderBranchInfo, FinderRemoteInfo, FinderRepoStatus,
    FinderStatus, FinderWorkspaceInfo, FinderWorktreeInfo, OrgFolderInfo,
    DEFAULT_IMPORTANT_IGNORED_PATTERNS,
};
use std::path::{Path, PathBuf};
use tracing::debug;

/// Service that scans repositories and computes badge status.
///
/// This is the core API. The daemon, CLI, and any future frontend
/// (HTTP server, native app) use this to get repository status.
pub struct RepoScanService<'a> {
    git: &'a dyn GitOperations,
    config: &'a Config,
}

impl<'a> RepoScanService<'a> {
    /// Create a new service bound to a git backend and config.
    pub fn new(git: &'a dyn GitOperations, config: &'a Config) -> Self {
        Self { git, config }
    }

    /// Scan all workspaces and build a complete `FinderStatus`.
    ///
    /// Used by: daemon loop, REFRESH_ALL socket command.
    pub fn scan_all(&self, pid: u32) -> Result<FinderStatus> {
        let timestamp = chrono::Utc::now().to_rfc3339();
        let mut status = FinderStatus::new(pid, timestamp);

        for ws_path in &self.config.workspaces {
            let expanded = shellexpand::tilde(ws_path).to_string();
            let root = PathBuf::from(&expanded);
            if !root.exists() {
                debug!(path = %root.display(), "Workspace root does not exist, skipping");
                continue;
            }

            // Load workspace config
            let ws_config = match WorkspaceStore::load(&root) {
                Ok(ws) => ws,
                Err(e) => {
                    debug!(
                        path = %root.display(),
                        error = %e,
                        "Failed to load workspace config, skipping"
                    );
                    continue;
                }
            };

            let base_path = ws_config.expanded_base_path();
            // Use directory name as workspace name
            let ws_name = base_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(ws_path)
                .to_string();

            // orgs is Vec<String> directly
            let org_names: Vec<String> = ws_config.orgs.clone();

            status.workspaces.push(FinderWorkspaceInfo {
                name: ws_name.clone(),
                root: base_path.clone(),
                orgs: org_names.clone(),
            });

            // Add org folder entries — scan filesystem for org directories
            // If orgs list is specified, use it; otherwise discover from directory listing
            let org_dirs: Vec<String> = if org_names.is_empty() {
                std::fs::read_dir(&base_path)
                    .ok()
                    .map(|entries| {
                        entries
                            .filter_map(|e| e.ok())
                            .filter(|e| e.file_type().map(|ft| ft.is_dir()).unwrap_or(false))
                            .filter(|e| {
                                e.file_name()
                                    .to_str()
                                    .map(|n| !n.starts_with('.'))
                                    .unwrap_or(false)
                            })
                            .filter_map(|e| e.file_name().into_string().ok())
                            .collect()
                    })
                    .unwrap_or_default()
            } else {
                org_names.clone()
            };

            for org_name in &org_dirs {
                let org_path = base_path.join(org_name);
                if org_path.exists() {
                    status.org_folders.push(OrgFolderInfo {
                        path: org_path,
                        org: org_name.clone(),
                        workspace: ws_name.clone(),
                    });
                }
            }

            // Scan local repos in this workspace
            let repos = self.scan_workspace_repos(&ws_config, Some(&ws_name));
            status.repos.extend(repos);
        }

        Ok(status)
    }

    /// Scan a single workspace and return its repos with full `FinderRepoStatus`.
    ///
    /// Used by: CLI `status` command.
    pub fn scan_workspace(&self, workspace: &WorkspaceConfig) -> Result<Vec<FinderRepoStatus>> {
        let base_path = workspace.expanded_base_path();
        let ws_name = base_path
            .file_name()
            .and_then(|n| n.to_str())
            .map(|n| n.to_string());

        Ok(self.scan_workspace_repos(workspace, ws_name.as_deref()))
    }

    /// Internal: scan all repos discovered inside a single workspace.
    fn scan_workspace_repos(
        &self,
        workspace: &WorkspaceConfig,
        workspace_name: Option<&str>,
    ) -> Vec<FinderRepoStatus> {
        let base_path = workspace.expanded_base_path();
        let structure = workspace
            .structure
            .as_deref()
            .unwrap_or(&self.config.structure);

        let orchestrator =
            DiscoveryOrchestrator::new(workspace.filters.clone(), structure.to_string());
        let local_repos = orchestrator.scan_local(&base_path, self.git);

        local_repos
            .into_iter()
            .map(|(repo_path, org, _name)| self.scan_repo(&repo_path, workspace_name, Some(&org)))
            .collect()
    }

    /// Scan a single repository and build its `FinderRepoStatus`.
    ///
    /// Used by: REFRESH /path socket command; internally by `scan_workspace_repos`.
    pub fn scan_repo(
        &self,
        repo_path: &Path,
        workspace: Option<&str>,
        org: Option<&str>,
    ) -> FinderRepoStatus {
        let git = self.git;

        // Get basic status
        let repo_status = git
            .status(repo_path)
            .unwrap_or_else(|_| crate::git::RepoStatus {
                branch: "unknown".to_string(),
                is_uncommitted: false,
                ahead: 0,
                behind: 0,
                has_untracked: false,
                staged_count: 0,
                unstaged_count: 0,
                untracked_count: 0,
            });

        // Get branches
        let branches: Vec<FinderBranchInfo> = git
            .list_branches(repo_path)
            .unwrap_or_default()
            .into_iter()
            .map(|b| FinderBranchInfo {
                name: b.name,
                upstream: b.upstream,
                ahead: b.ahead,
                behind: b.behind,
                synced: b.is_synced,
            })
            .collect();

        let all_branches_synced = branches.iter().all(|b| b.synced);

        // Get remotes
        let remotes: Vec<FinderRemoteInfo> = git
            .list_remotes(repo_path)
            .unwrap_or_default()
            .into_iter()
            .map(|r| FinderRemoteInfo {
                name: r.name,
                url: r.fetch_url,
            })
            .collect();

        // Get worktrees
        let worktree_infos = git.list_worktrees(repo_path).unwrap_or_default();
        let mut worktrees = Vec::new();
        let mut all_worktrees_synced = true;

        for wt in &worktree_infos {
            // Skip the main worktree (same as repo_path)
            if wt.path == repo_path {
                continue;
            }
            // Check worktree status
            let wt_synced = if wt.is_bare || wt.is_detached {
                true
            } else {
                git.status(&wt.path)
                    .map(|s| s.is_clean_and_synced())
                    .unwrap_or(false)
            };
            if !wt_synced {
                all_worktrees_synced = false;
            }
            worktrees.push(FinderWorktreeInfo {
                path: wt.path.clone(),
                branch: wt.branch.clone(),
                synced: wt_synced,
            });
        }

        // Get commit count
        let commit_count = git.commit_count(repo_path).unwrap_or(0);

        // Get stash count
        let stash_count = git.stash_count(repo_path).unwrap_or(0);

        // Check for important ignored files (only if otherwise clean)
        let is_otherwise_clean = repo_status.staged_count == 0
            && repo_status.unstaged_count == 0
            && repo_status.untracked_count == 0
            && repo_status.ahead == 0
            && all_branches_synced
            && all_worktrees_synced;

        let (has_important_ignored_files, important_ignored_files) = if is_otherwise_clean {
            self.check_important_ignored(repo_path)
        } else {
            (false, Vec::new())
        };

        // Compute badge
        let badge = compute_badge(
            repo_status.staged_count,
            repo_status.unstaged_count,
            repo_status.untracked_count,
            repo_status.ahead,
            all_branches_synced,
            all_worktrees_synced,
            has_important_ignored_files,
        );

        FinderRepoStatus {
            path: repo_path.to_path_buf(),
            workspace: workspace.map(|s| s.to_string()),
            org: org.map(|s| s.to_string()),
            badge,
            current_branch: repo_status.branch,
            default_branch: None,
            commit_count,
            staged_count: repo_status.staged_count,
            unstaged_count: repo_status.unstaged_count,
            untracked_count: repo_status.untracked_count,
            ahead: repo_status.ahead,
            behind: repo_status.behind,
            stash_count,
            has_important_ignored_files,
            important_ignored_files,
            branches,
            all_branches_synced,
            remotes,
            worktrees,
            all_worktrees_synced,
        }
    }

    /// Check if a repo has important ignored files matching the configured patterns.
    fn check_important_ignored(&self, repo_path: &Path) -> (bool, Vec<String>) {
        let ignored_files = match self.git.list_ignored_files(repo_path) {
            Ok(files) => files,
            Err(_) => return (false, Vec::new()),
        };

        let patterns = DEFAULT_IMPORTANT_IGNORED_PATTERNS;
        let important: Vec<String> = ignored_files
            .into_iter()
            .filter(|f| matches_important_pattern(f, patterns))
            .collect();

        let has_any = !important.is_empty();
        (has_any, important)
    }
}

#[cfg(test)]
#[path = "service_tests.rs"]
mod tests;
