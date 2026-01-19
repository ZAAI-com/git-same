//! Discovery orchestration module.
//!
//! This module coordinates repository discovery across providers
//! and manages action planning for clone/sync operations.

use crate::config::FilterOptions;
use crate::git::GitOperations;
use crate::provider::{DiscoveryOptions, DiscoveryProgress, Provider};
use crate::sync::LocalRepo;
use crate::types::{ActionPlan, OwnedRepo};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Orchestrates repository discovery.
pub struct DiscoveryOrchestrator {
    /// Filter options
    filters: FilterOptions,
    /// Directory structure template
    structure: String,
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
        let path_str = self
            .structure
            .replace("{provider}", provider)
            .replace("{org}", &repo.owner)
            .replace("{repo}", &repo.repo.name);

        base_path.join(path_str)
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

        self.scan_dir(base_path, git, &mut repos, 0, depth);

        repos
    }

    /// Recursively scans directories for git repos.
    fn scan_dir<G: GitOperations>(
        &self,
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
            if entry
                .file_name()
                .to_string_lossy()
                .starts_with('.')
            {
                continue;
            }

            if current_depth + 1 == max_depth && git.is_repo(&entry_path) {
                // This is a repo at the expected depth
                let rel_path = entry_path.strip_prefix(path).unwrap_or(&entry_path);
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
                self.scan_dir(&entry_path, git, repos, current_depth + 1, max_depth);
            }
        }
    }
}

/// Merges discovered repos from multiple providers.
pub fn merge_repos(
    repos_by_provider: Vec<(String, Vec<OwnedRepo>)>,
) -> Vec<(String, OwnedRepo)> {
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
mod tests {
    use super::*;
    use crate::git::MockGit;
    use crate::types::Repo;
    use tempfile::TempDir;

    fn test_repo(name: &str, owner: &str) -> OwnedRepo {
        OwnedRepo::new(owner, Repo::test(name, owner))
    }

    #[test]
    fn test_orchestrator_creation() {
        let filters = FilterOptions::default();
        let orchestrator = DiscoveryOrchestrator::new(filters, "{org}/{repo}".to_string());
        assert_eq!(orchestrator.structure, "{org}/{repo}");
    }

    #[test]
    fn test_compute_path_simple() {
        let filters = FilterOptions::default();
        let orchestrator = DiscoveryOrchestrator::new(filters, "{org}/{repo}".to_string());

        let repo = test_repo("my-repo", "my-org");
        let path = orchestrator.compute_path(Path::new("/base"), &repo, "github");

        assert_eq!(path, PathBuf::from("/base/my-org/my-repo"));
    }

    #[test]
    fn test_compute_path_with_provider() {
        let filters = FilterOptions::default();
        let orchestrator =
            DiscoveryOrchestrator::new(filters, "{provider}/{org}/{repo}".to_string());

        let repo = test_repo("my-repo", "my-org");
        let path = orchestrator.compute_path(Path::new("/base"), &repo, "github");

        assert_eq!(path, PathBuf::from("/base/github/my-org/my-repo"));
    }

    #[test]
    fn test_plan_clone_new_repos() {
        let filters = FilterOptions::default();
        let orchestrator = DiscoveryOrchestrator::new(filters, "{org}/{repo}".to_string());
        let git = MockGit::new();

        let repos = vec![
            test_repo("repo1", "org"),
            test_repo("repo2", "org"),
        ];

        let plan = orchestrator.plan_clone(Path::new("/nonexistent"), repos, "github", &git);

        assert_eq!(plan.to_clone.len(), 2);
        assert_eq!(plan.to_sync.len(), 0);
        assert_eq!(plan.skipped.len(), 0);
    }

    #[test]
    fn test_plan_clone_existing_repos() {
        let temp = TempDir::new().unwrap();
        let repo_path = temp.path().join("org/repo");
        std::fs::create_dir_all(&repo_path).unwrap();

        let mut git = MockGit::new();
        git.add_repo(repo_path.to_string_lossy().to_string());

        let filters = FilterOptions::default();
        let orchestrator = DiscoveryOrchestrator::new(filters, "{org}/{repo}".to_string());

        let repos = vec![test_repo("repo", "org")];
        let plan = orchestrator.plan_clone(temp.path(), repos, "github", &git);

        assert_eq!(plan.to_clone.len(), 0);
        assert_eq!(plan.to_sync.len(), 1);
        assert_eq!(plan.skipped.len(), 0);
    }

    #[test]
    fn test_plan_clone_non_repo_dir() {
        let temp = TempDir::new().unwrap();
        let repo_path = temp.path().join("org/repo");
        std::fs::create_dir_all(&repo_path).unwrap();

        let git = MockGit::new(); // Not marked as a repo

        let filters = FilterOptions::default();
        let orchestrator = DiscoveryOrchestrator::new(filters, "{org}/{repo}".to_string());

        let repos = vec![test_repo("repo", "org")];
        let plan = orchestrator.plan_clone(temp.path(), repos, "github", &git);

        assert_eq!(plan.to_clone.len(), 0);
        assert_eq!(plan.to_sync.len(), 0);
        assert_eq!(plan.skipped.len(), 1);
    }

    #[test]
    fn test_plan_sync() {
        let temp = TempDir::new().unwrap();
        let repo_path = temp.path().join("org/repo");
        std::fs::create_dir_all(&repo_path).unwrap();

        let mut git = MockGit::new();
        git.add_repo(repo_path.to_string_lossy().to_string());

        let filters = FilterOptions::default();
        let orchestrator = DiscoveryOrchestrator::new(filters, "{org}/{repo}".to_string());

        let repos = vec![test_repo("repo", "org")];
        let (to_sync, skipped) =
            orchestrator.plan_sync(temp.path(), repos, "github", &git, false);

        assert_eq!(to_sync.len(), 1);
        assert_eq!(skipped.len(), 0);
    }

    #[test]
    fn test_plan_sync_not_cloned() {
        let filters = FilterOptions::default();
        let orchestrator = DiscoveryOrchestrator::new(filters, "{org}/{repo}".to_string());
        let git = MockGit::new();

        let repos = vec![test_repo("repo", "org")];
        let (to_sync, skipped) =
            orchestrator.plan_sync(Path::new("/nonexistent"), repos, "github", &git, false);

        assert_eq!(to_sync.len(), 0);
        assert_eq!(skipped.len(), 1);
        assert!(skipped[0].1.contains("not cloned"));
    }

    #[test]
    fn test_merge_repos() {
        let repos1 = vec![test_repo("repo1", "org1")];
        let repos2 = vec![test_repo("repo2", "org2")];

        let merged = merge_repos(vec![
            ("github".to_string(), repos1),
            ("gitlab".to_string(), repos2),
        ]);

        assert_eq!(merged.len(), 2);
        assert_eq!(merged[0].0, "github");
        assert_eq!(merged[1].0, "gitlab");
    }

    #[test]
    fn test_deduplicate_repos() {
        let repo1 = test_repo("repo", "org");
        let repo2 = test_repo("repo", "org"); // Duplicate

        let repos = vec![
            ("github".to_string(), repo1),
            ("gitlab".to_string(), repo2),
        ];

        let deduped = deduplicate_repos(repos);
        assert_eq!(deduped.len(), 1);
        assert_eq!(deduped[0].0, "github"); // First one wins
    }

    #[test]
    fn test_to_discovery_options() {
        let filters = FilterOptions {
            include_archived: true,
            include_forks: false,
            orgs: vec!["org1".to_string(), "org2".to_string()],
            exclude_repos: vec!["org/skip-this".to_string()],
        };

        let orchestrator = DiscoveryOrchestrator::new(filters.clone(), "{org}/{repo}".to_string());
        let options = orchestrator.to_discovery_options();

        assert!(options.include_archived);
        assert!(!options.include_forks);
        assert_eq!(options.org_filter, vec!["org1", "org2"]);
    }
}
