//! Provider-side discovery behavior.

use super::DiscoveryOrchestrator;
use crate::provider::{DiscoveryOptions, DiscoveryProgress, Provider};
use crate::types::OwnedRepo;
use std::path::{Path, PathBuf};

impl DiscoveryOrchestrator {
    /// Creates a new discovery orchestrator.
    pub fn new(filters: crate::config::FilterOptions, structure: String) -> Self {
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
}
