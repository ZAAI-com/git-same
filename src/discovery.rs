//! Discovery orchestration module.
//!
//! This module coordinates repository discovery across providers
//! and manages action planning for clone/sync operations.

use crate::config::FilterOptions;
use crate::domain::RepoPathTemplate;
use crate::git::GitOperations;
use crate::operations::sync::LocalRepo;
use crate::provider::{DiscoveryOptions, DiscoveryProgress, Provider};
use crate::types::{ActionPlan, OwnedRepo};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Mutable context for directory scanning (keeps `scan_dir` under Clippy’s argument limit).
struct ScanDirContext<'a, G: GitOperations + ?Sized> {
    base_path: &'a Path,
    git: &'a G,
    repos: &'a mut Vec<(PathBuf, String, String)>,
    visited_dirs: &'a mut HashSet<PathBuf>,
    seen_repos: &'a mut HashSet<PathBuf>,
    max_depth: usize,
}

/// Orchestrates repository discovery.
pub struct DiscoveryOrchestrator {
    /// Filter options
    pub(crate) filters: FilterOptions,
    /// Directory structure template
    pub(crate) structure: String,
}

impl DiscoveryOrchestrator {
    /// Creates a new discovery orchestrator.
    pub fn new(filters: FilterOptions, structure: String) -> Self {
        Self { filters, structure }
    }

    /// Converts filter options to discovery options.
    pub fn to_discovery_options(&self) -> DiscoveryOptions {
        DiscoveryOptions::new()
            .with_archived(self.filters.include_archived)
            .with_forks(self.filters.include_forks)
            .with_orgs(self.filters.orgs.clone())
            .with_exclusions(self.filters.exclude_repos.clone())
    }

    /// Discovers repositories from a provider.
    pub async fn discover(
        &self,
        provider: &dyn Provider,
        progress: &dyn DiscoveryProgress,
    ) -> Result<Vec<OwnedRepo>, crate::errors::ProviderError> {
        let options = self.to_discovery_options();
        provider.discover_repos(&options, progress).await
    }

    /// Computes the local path for a repository.
    pub fn compute_path(&self, base_path: &Path, repo: &OwnedRepo, provider: &str) -> PathBuf {
        RepoPathTemplate::new(self.structure.clone()).render_owned_repo(base_path, repo, provider)
    }

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
        skip_uncommitted: bool,
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

            if skip_uncommitted {
                match git.status(&local_path) {
                    Ok(status) => {
                        if status.is_uncommitted || status.has_untracked {
                            skipped.push((repo, "uncommitted changes".to_string()));
                            continue;
                        }
                    }
                    Err(err) => {
                        skipped.push((repo, format!("failed to get status: {}", err)));
                        continue;
                    }
                }
            }

            to_sync.push(LocalRepo::new(repo, local_path));
        }

        (to_sync, skipped)
    }

    /// Scans local filesystem for cloned repositories.
    pub fn scan_local<G: GitOperations + ?Sized>(
        &self,
        base_path: &Path,
        git: &G,
    ) -> Vec<(PathBuf, String, String)> {
        let mut repos = Vec::new();
        let mut visited_dirs = HashSet::new();
        let mut seen_repos = HashSet::new();

        // Determine scan depth based on structure
        // {org}/{repo} -> 2 levels
        // {provider}/{org}/{repo} -> 3 levels
        let depth = RepoPathTemplate::new(self.structure.clone()).scan_depth();
        let mut ctx = ScanDirContext {
            base_path,
            git,
            repos: &mut repos,
            visited_dirs: &mut visited_dirs,
            seen_repos: &mut seen_repos,
            max_depth: depth,
        };
        self.scan_dir(base_path, 0, &mut ctx);

        repos
    }

    /// Recursively scans directories for git repos.
    fn scan_dir<G: GitOperations + ?Sized>(
        &self,
        path: &Path,
        current_depth: usize,
        ctx: &mut ScanDirContext<'_, G>,
    ) {
        if current_depth >= ctx.max_depth {
            return;
        }

        let canonical_path = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
        if !ctx.visited_dirs.insert(canonical_path.clone()) {
            return;
        }

        let entries = match std::fs::read_dir(&canonical_path) {
            Ok(e) => e,
            Err(_) => return,
        };

        for entry in entries.flatten() {
            // Avoid traversing symlinks to directories.
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if !file_type.is_dir() {
                continue;
            }

            let entry_path = entry.path();

            // Skip hidden directories
            if entry.file_name().to_string_lossy().starts_with('.') {
                continue;
            }

            if current_depth + 1 == ctx.max_depth && ctx.git.is_repo(&entry_path) {
                let canonical_repo =
                    std::fs::canonicalize(&entry_path).unwrap_or(entry_path.clone());
                if !ctx.seen_repos.insert(canonical_repo.clone()) {
                    continue;
                }

                // This is a repo at the expected depth
                let rel_path = canonical_repo
                    .strip_prefix(ctx.base_path)
                    .unwrap_or(&canonical_repo);
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
                    ctx.repos.push((canonical_repo, org, repo));
                }
            } else {
                // Recurse into subdirectory
                self.scan_dir(&entry_path, current_depth + 1, ctx);
            }
        }
    }
}

/// Walks the given roots and returns every git repository root found.
///
/// A directory is a repo if it contains a `.git` entry (directory for normal
/// repos, or a file for worktree/submodule gitlinks). Once a repo is found we
/// stop descending into it — we only care about repo roots, not files inside.
///
/// - `max_depth` caps recursion depth (the root itself is depth 0).
/// - `exclude` is matched against directory **names** (not full paths), so
///   passing `"node_modules"` skips every `node_modules/` no matter where it
///   sits. Pass lowercase for case-insensitive matching if needed; current
///   behaviour is exact match.
/// - Symlinks and cycles are handled via a visited-set of canonical paths.
/// - Permission-denied or I/O errors are silently skipped.
///
/// Returns canonical paths in the order they were discovered, deduplicated.
pub fn find_git_repos(
    roots: &[PathBuf],
    max_depth: usize,
    exclude: &HashSet<String>,
) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = Vec::new();
    let mut found_set: HashSet<PathBuf> = HashSet::new();
    let mut visited: HashSet<PathBuf> = HashSet::new();

    for root in roots {
        let Ok(canonical_root) = std::fs::canonicalize(root) else {
            continue;
        };
        walk_for_repos(
            &canonical_root,
            0,
            max_depth,
            exclude,
            &mut found,
            &mut found_set,
            &mut visited,
        );
    }

    found
}

fn walk_for_repos(
    path: &Path,
    depth: usize,
    max_depth: usize,
    exclude: &HashSet<String>,
    found: &mut Vec<PathBuf>,
    found_set: &mut HashSet<PathBuf>,
    visited: &mut HashSet<PathBuf>,
) {
    if !visited.insert(path.to_path_buf()) {
        return;
    }

    // A repo is identified by the presence of a `.git` entry (dir or file).
    // `symlink_metadata` is cheaper than `metadata` and doesn't follow links.
    if std::fs::symlink_metadata(path.join(".git")).is_ok() {
        if found_set.insert(path.to_path_buf()) {
            found.push(path.to_path_buf());
        }
        return; // don't descend into a repo's own tree
    }

    if depth >= max_depth {
        return;
    }

    let Ok(entries) = std::fs::read_dir(path) else {
        return;
    };

    for entry in entries.flatten() {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if !file_type.is_dir() {
            continue;
        }

        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        // Skip excluded names and hidden dirs (hidden dirs rarely contain
        // repos we care about; `.git-same`, `.Trash` etc. already excluded
        // by name list but this is a belt-and-braces).
        if exclude.contains(name_str.as_ref()) {
            continue;
        }
        if name_str.starts_with('.') {
            continue;
        }

        let child = entry.path();
        let canonical_child = std::fs::canonicalize(&child).unwrap_or(child);
        walk_for_repos(
            &canonical_child,
            depth + 1,
            max_depth,
            exclude,
            found,
            found_set,
            visited,
        );
    }
}

/// Merges discovered repos from multiple providers.
pub fn merge_repos(repos_by_provider: Vec<(String, Vec<OwnedRepo>)>) -> Vec<(String, OwnedRepo)> {
    let mut result = Vec::new();

    for (provider, repos) in repos_by_provider {
        for repo in repos {
            result.push((provider.clone(), repo));
        }
    }

    result
}

/// Deduplicates repos by full name, preferring first occurrence.
pub fn deduplicate_repos(repos: Vec<(String, OwnedRepo)>) -> Vec<(String, OwnedRepo)> {
    let mut seen = HashSet::new();
    let mut result = Vec::new();

    for (provider, repo) in repos {
        let key = repo.full_name().to_string();
        if !seen.contains(&key) {
            seen.insert(key);
            result.push((provider, repo));
        }
    }

    result
}

#[cfg(test)]
#[path = "discovery_tests.rs"]
mod tests;
