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
                if let Ok(status) = git.status(&local_path) {
                    if status.is_uncommitted || status.has_untracked {
                        skipped.push((repo, "uncommitted changes".to_string()));
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
        let depth = RepoPathTemplate::new(self.structure.clone()).scan_depth();

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
