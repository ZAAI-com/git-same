//! Local planning and filesystem scanning behavior.

use super::DiscoveryOrchestrator;
use crate::core::operations::sync::LocalRepo;
use crate::git::GitOperations;
use crate::types::{ActionPlan, OwnedRepo};
use std::path::{Path, PathBuf};

impl DiscoveryOrchestrator {
    /// Creates an action plan by comparing discovered repos with local filesystem.
    pub fn plan_clone<G: GitOperations>(
        &self,
        base_path: &Path,
        repos: Vec<OwnedRepo>,
        provider: &str,
        git: &G,
    ) -> ActionPlan {
        let mut plan = ActionPlan::new();

        for repo in repos {
            let local_path = self.compute_path(base_path, &repo, provider);

            if local_path.exists() {
                if git.is_repo(&local_path) {
                    // Existing repo - add to sync
                    plan.add_sync(repo);
                } else {
                    // Directory exists but not a repo
                    plan.add_skipped(repo, "directory exists but is not a git repository");
                }
            } else {
                // New repo - add to clone
                plan.add_clone(repo);
            }
        }

        plan
    }

    /// Creates a sync plan for existing local repositories.
    pub fn plan_sync<G: GitOperations>(
        &self,
        base_path: &Path,
        repos: Vec<OwnedRepo>,
        provider: &str,
        git: &G,
        skip_dirty: bool,
    ) -> (Vec<LocalRepo>, Vec<(OwnedRepo, String)>) {
        let mut to_sync = Vec::new();
        let mut skipped = Vec::new();

        for repo in repos {
            let local_path = self.compute_path(base_path, &repo, provider);

            if !local_path.exists() {
                skipped.push((repo, "not cloned locally".to_string()));
                continue;
            }

            if !git.is_repo(&local_path) {
                skipped.push((repo, "not a git repository".to_string()));
                continue;
            }

            if skip_dirty {
                if let Ok(status) = git.status(&local_path) {
                    if status.is_dirty || status.has_untracked {
                        skipped.push((repo, "working tree is dirty".to_string()));
                        continue;
                    }
                }
            }

            to_sync.push(LocalRepo::new(repo, local_path));
        }

        (to_sync, skipped)
    }

    /// Scans local filesystem for cloned repositories.
    pub fn scan_local<G: GitOperations>(
        &self,
        base_path: &Path,
        git: &G,
    ) -> Vec<(PathBuf, String, String)> {
        let mut repos = Vec::new();

        // Determine scan depth based on structure
        // {org}/{repo} -> 2 levels
        // {provider}/{org}/{repo} -> 3 levels
        let has_provider = self.structure.contains("{provider}");
        let depth = if has_provider { 3 } else { 2 };

        self.scan_dir(base_path, base_path, git, &mut repos, 0, depth);

        repos
    }

    /// Recursively scans directories for git repos.
    fn scan_dir<G: GitOperations>(
        &self,
        base_path: &Path,
        path: &Path,
        git: &G,
        repos: &mut Vec<(PathBuf, String, String)>,
        current_depth: usize,
        max_depth: usize,
    ) {
        if current_depth >= max_depth {
            return;
        }

        let entries = match std::fs::read_dir(path) {
            Ok(e) => e,
            Err(_) => return,
        };

        for entry in entries.flatten() {
            let entry_path = entry.path();
            if !entry_path.is_dir() {
                continue;
            }

            // Skip hidden directories
            if entry.file_name().to_string_lossy().starts_with('.') {
                continue;
            }

            if current_depth + 1 == max_depth && git.is_repo(&entry_path) {
                // This is a repo at the expected depth
                let rel_path = entry_path.strip_prefix(base_path).unwrap_or(&entry_path);
                let parts: Vec<_> = rel_path.components().collect();

                if parts.len() >= 2 {
                    let org = parts[parts.len() - 2]
                        .as_os_str()
                        .to_string_lossy()
                        .to_string();
                    let repo = parts[parts.len() - 1]
                        .as_os_str()
                        .to_string_lossy()
                        .to_string();
                    repos.push((entry_path.clone(), org, repo));
                }
            } else {
                // Recurse into subdirectory
                self.scan_dir(
                    base_path,
                    &entry_path,
                    git,
                    repos,
                    current_depth + 1,
                    max_depth,
                );
            }
        }
    }
}
