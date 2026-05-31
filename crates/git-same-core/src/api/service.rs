//! Repository scanning service.
//!
//! `RepoScanService` is the API for scanning repositories and computing badge
//! status. It owns no state — callers construct it with references to a git
//! backend and a config, then invoke `scan_all()`, `scan_workspace()`, or
//! `scan_repo()`.

use crate::api::{AmbientUpgradeCache, OwnerTypeCache};
use crate::config::{Config, WorkspaceConfig, WorkspaceStore};
use crate::discovery::{find_git_repos, DiscoveryOrchestrator};
use crate::errors::Result;
use crate::git::GitOperations;
use crate::types::finder_status::{
    compute_badge, matches_important_pattern, Badge, FinderBranchInfo, FinderRemoteInfo,
    FinderRepoStatus, FinderStatus, FinderWorkspaceInfo, FinderWorktreeInfo, OrgFolderInfo,
    OwnerType, DEFAULT_IMPORTANT_IGNORED_PATTERNS,
};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use tracing::debug;

/// Service that scans repositories and computes badge status.
///
/// This is the core API. The monitor, CLI, and any future frontend
/// (HTTP server, native app) use this to get repository status.
pub struct RepoScanService<'a> {
    git: &'a dyn GitOperations,
    config: &'a Config,
    owner_types: Option<OwnerTypeCache>,
    ambient_upgrades: Option<AmbientUpgradeCache>,
}

impl<'a> RepoScanService<'a> {
    /// Create a new service bound to a git backend and config.
    pub fn new(git: &'a dyn GitOperations, config: &'a Config) -> Self {
        Self {
            git,
            config,
            owner_types: None,
            ambient_upgrades: None,
        }
    }

    /// Attach an owner-type cache so scanned org folders are annotated with
    /// `OwnerType::User` / `OwnerType::Organization`.
    pub fn with_owner_types(mut self, cache: OwnerTypeCache) -> Self {
        self.owner_types = Some(cache);
        self
    }

    /// Attach an ambient-upgrade cache so previously-upgraded ambient repos
    /// keep their semantic color across periodic rescans.
    pub fn with_ambient_upgrades(mut self, cache: AmbientUpgradeCache) -> Self {
        self.ambient_upgrades = Some(cache);
        self
    }

    /// Clone the attached owner-type cache handle, if any, so socket-handler
    /// tasks can reuse the same cache.
    pub fn owner_types_clone(&self) -> Option<OwnerTypeCache> {
        self.owner_types.clone()
    }

    /// Clone the attached ambient-upgrade cache handle, if any, so socket-handler
    /// tasks can reuse it.
    pub fn ambient_upgrades_clone(&self) -> Option<AmbientUpgradeCache> {
        self.ambient_upgrades.clone()
    }

    /// Scan all workspaces and build a complete `FinderStatus`.
    ///
    /// Used by: monitor loop, REFRESH_ALL socket command.
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

            // Include the configured `username` alongside the orgs list so the
            // user's own GitHub login gets a folder entry (and a "U" badge) even
            // if it isn't in the org allowlist.
            let mut owner_dirs: Vec<(String, OwnerType)> = org_dirs
                .iter()
                .map(|n| (n.clone(), OwnerType::Unknown))
                .collect();
            if !ws_config.username.is_empty()
                && !owner_dirs.iter().any(|(n, _)| n == &ws_config.username)
            {
                owner_dirs.push((ws_config.username.clone(), OwnerType::User));
            }

            for (owner_name, known_type) in &owner_dirs {
                let owner_path = base_path.join(owner_name);
                if owner_path.exists() {
                    let cached = self.owner_types.as_ref().and_then(|c| c.get(owner_name));
                    let owner_type = cached.unwrap_or(*known_type);
                    status.org_folders.push(OrgFolderInfo {
                        path: owner_path,
                        org: owner_name.clone(),
                        workspace: ws_name.clone(),
                        owner_type,
                    });
                }
            }

            // Scan local repos in this workspace
            let repos = self.scan_workspace_repos(&ws_config, Some(&ws_name));
            status.repos.extend(repos);
        }

        self.populate_ambient(&mut status);

        Ok(status)
    }

    /// Append ambient (non-workspace) repos and populate `monitored_roots`.
    ///
    /// Monitored roots = workspace roots ∪ `finder.scan_roots`. The extension
    /// uses this union as its `FIFinderSyncController.directoryURLs`.
    fn populate_ambient(&self, status: &mut FinderStatus) {
        // Publish boot-volume aliases so the sandboxed extension can map
        // alias-presented Finder paths to canonical keys with pure string
        // ops (no filesystem access). Always set, independent of ambient
        // mode, since workspace roots can also be browsed through the alias.
        status.boot_volume_aliases = detect_boot_volume_aliases();

        // Always publish workspace roots so the extension can register them.
        for ws in &status.workspaces {
            if !status.monitored_roots.contains(&ws.root) {
                status.monitored_roots.push(ws.root.clone());
            }
        }

        if !self.config.finder.show_ambient {
            return;
        }

        let scan_roots: Vec<PathBuf> = self
            .config
            .finder
            .scan_roots
            .iter()
            .map(|s| shellexpand::tilde(s).to_string())
            .map(PathBuf::from)
            .filter(|p| p.exists())
            .collect();

        for root in &scan_roots {
            let canonical = std::fs::canonicalize(root).unwrap_or_else(|_| root.clone());
            if !status.monitored_roots.contains(&canonical) {
                status.monitored_roots.push(canonical);
            }
        }

        let exclude: HashSet<String> = self.config.finder.exclude_dirs.iter().cloned().collect();
        let ambient_paths = find_git_repos(&scan_roots, self.config.finder.max_depth, &exclude);

        // Dedupe against already-emitted workspace repos (canonical form).
        let workspace_paths: HashSet<PathBuf> = status
            .repos
            .iter()
            .map(|r| std::fs::canonicalize(&r.path).unwrap_or_else(|_| r.path.clone()))
            .collect();

        for path in ambient_paths {
            if workspace_paths.contains(&path) {
                continue;
            }

            // Upgraded ambient repos stay upgraded until the monitor exits
            // or the repo disappears.
            let entry = self
                .ambient_upgrades
                .as_ref()
                .and_then(|cache| {
                    if !path.join(".git").exists() {
                        cache.remove(&path);
                        return None;
                    }
                    cache.get(&path)
                })
                .unwrap_or_else(|| self.scan_ambient_repo(&path));

            status.repos.push(entry);
        }
    }

    /// Build a minimal `FinderRepoStatus` for an ambient (non-workspace) repo.
    ///
    /// Intentionally performs zero git I/O: the user only needs to *spot* the
    /// repo. Full status is computed on demand when the right-click menu
    /// triggers a `REFRESH /path`.
    pub fn scan_ambient_repo(&self, path: &Path) -> FinderRepoStatus {
        FinderRepoStatus {
            path: path.to_path_buf(),
            workspace: None,
            org: None,
            badge: Badge::Gray,
            current_branch: String::new(),
            default_branch: None,
            commit_count: 0,
            staged_count: 0,
            unstaged_count: 0,
            untracked_count: 0,
            ahead: 0,
            behind: 0,
            stash_count: 0,
            has_important_ignored_files: false,
            important_ignored_files: Vec::new(),
            branches: Vec::new(),
            all_branches_synced: true,
            remotes: Vec::new(),
            worktrees: Vec::new(),
            all_worktrees_synced: true,
            read_error: None,
        }
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

        // Get basic status. If `git status` fails, capture the error so the
        // CLI can warn the user and we can force the badge to Gray instead
        // of letting the all-zero defaults masquerade as a clean repo.
        let (repo_status, read_error) = match git.status(repo_path) {
            Ok(s) => (s, None),
            Err(e) => (
                crate::git::RepoStatus {
                    branch: "unknown".to_string(),
                    is_uncommitted: false,
                    ahead: 0,
                    behind: 0,
                    has_untracked: false,
                    staged_count: 0,
                    unstaged_count: 0,
                    untracked_count: 0,
                },
                Some(e.to_string()),
            ),
        };

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

        // Compute badge. Unreadable repos stay Gray so they don't pose as
        // healthy Green repos in the Finder or in `gisa status`.
        let badge = if read_error.is_some() {
            Badge::Gray
        } else {
            compute_badge(
                repo_status.staged_count,
                repo_status.unstaged_count,
                repo_status.untracked_count,
                repo_status.ahead,
                all_branches_synced,
                all_worktrees_synced,
                has_important_ignored_files,
            )
        };

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
            read_error,
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

/// Detect boot-volume aliases: `/Volumes/<name>` entries that are symlinks
/// pointing at the root volume `/`. macOS auto-creates one of these so Finder
/// can show the boot volume by name; folders browsed through it keep the
/// `/Volumes/<name>` prefix in their URL. The monitor (non-sandboxed) reads
/// `/Volumes` and publishes the prefixes so the sandboxed extension can do
/// alias→canonical mapping with pure string ops instead of touching the disk.
///
/// Returns an empty vec on any I/O error (the common no-alias case).
fn detect_boot_volume_aliases() -> Vec<String> {
    let entries = match std::fs::read_dir("/Volumes") {
        Ok(entries) => entries,
        Err(_) => return Vec::new(),
    };
    let mut aliases = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        // Only symlinks whose target is exactly the root volume "/".
        match std::fs::read_link(&path) {
            Ok(target) if target == Path::new("/") => {
                if let Some(s) = path.to_str() {
                    aliases.push(s.to_string());
                }
            }
            _ => {}
        }
    }
    aliases
}

#[cfg(test)]
#[path = "service_tests.rs"]
mod tests;
