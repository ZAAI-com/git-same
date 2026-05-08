//! Provider trait definitions.
//!
//! The [`Provider`] trait defines the interface that all Git hosting
//! provider implementations must implement.

use async_trait::async_trait;

use crate::errors::ProviderError;
use crate::types::{Org, OwnedRepo, OwnerType, ProviderKind, Repo};

/// Authentication credentials for a provider.
#[derive(Debug, Clone)]
pub struct Credentials {
    /// The authentication token
    pub token: String,
    /// Base URL for API calls
    pub api_base_url: String,
    /// The authenticated username (if known)
    pub username: Option<String>,
}

impl Credentials {
    /// Creates new credentials with token and API URL.
    pub fn new(token: impl Into<String>, api_base_url: impl Into<String>) -> Self {
        Self {
            token: token.into(),
            api_base_url: api_base_url.into(),
            username: None,
        }
    }

    /// Sets the username.
    pub fn with_username(mut self, username: impl Into<String>) -> Self {
        self.username = Some(username.into());
        self
    }
}

/// Rate limit information from the provider.
#[derive(Debug, Clone, Default)]
pub struct RateLimitInfo {
    /// Maximum requests allowed per period
    pub limit: u32,
    /// Remaining requests in current period
    pub remaining: u32,
    /// Unix timestamp when the limit resets
    pub reset_at: Option<i64>,
}

impl RateLimitInfo {
    /// Returns true if the rate limit is exhausted.
    pub fn is_exhausted(&self) -> bool {
        self.remaining == 0
    }

    /// Returns the number of seconds until the rate limit resets.
    pub fn seconds_until_reset(&self) -> Option<i64> {
        self.reset_at.map(|reset| {
            let now = chrono::Utc::now().timestamp();
            (reset - now).max(0)
        })
    }
}

/// Options for repository discovery.
#[derive(Debug, Clone, Default)]
pub struct DiscoveryOptions {
    /// Include archived repositories
    pub include_archived: bool,
    /// Include forked repositories
    pub include_forks: bool,
    /// Filter to specific organizations (empty = all)
    pub org_filter: Vec<String>,
    /// Exclude specific repos by full name
    pub exclude_repos: Vec<String>,
}

impl DiscoveryOptions {
    /// Creates default discovery options.
    pub fn new() -> Self {
        Self::default()
    }

    /// Include archived repositories.
    pub fn with_archived(mut self, include: bool) -> Self {
        self.include_archived = include;
        self
    }

    /// Include forked repositories.
    pub fn with_forks(mut self, include: bool) -> Self {
        self.include_forks = include;
        self
    }

    /// Filter to specific organizations.
    pub fn with_orgs(mut self, orgs: Vec<String>) -> Self {
        self.org_filter = orgs;
        self
    }

    /// Exclude specific repositories.
    pub fn with_exclusions(mut self, repos: Vec<String>) -> Self {
        self.exclude_repos = repos;
        self
    }

    /// Check if a repo should be included based on filters.
    pub fn should_include(&self, repo: &Repo) -> bool {
        // Check archived filter
        if !self.include_archived && repo.archived {
            return false;
        }

        // Check fork filter
        if !self.include_forks && repo.fork {
            return false;
        }

        // Check exclusion list
        if self.exclude_repos.contains(&repo.full_name) {
            return false;
        }

        true
    }

    /// Check if an org should be included based on filters.
    pub fn should_include_org(&self, org: &str) -> bool {
        if self.org_filter.is_empty() {
            return true;
        }
        self.org_filter.iter().any(|o| o == org)
    }
}

/// Callback trait for progress reporting during discovery.
pub trait DiscoveryProgress: Send + Sync {
    /// Called when organizations are discovered.
    fn on_orgs_discovered(&self, count: usize);

    /// Called when starting to fetch repos for an org.
    fn on_org_started(&self, org_name: &str);

    /// Called when finished fetching repos for an org.
    fn on_org_complete(&self, org_name: &str, repo_count: usize);

    /// Called when starting to fetch personal repos.
    fn on_personal_repos_started(&self);

    /// Called when finished fetching personal repos.
    fn on_personal_repos_complete(&self, count: usize);

    /// Called on any error during discovery (non-fatal).
    fn on_error(&self, message: &str);
}

/// A no-op implementation for when progress isn't needed.
#[derive(Debug, Default)]
pub struct NoProgress;

impl DiscoveryProgress for NoProgress {
    fn on_orgs_discovered(&self, _: usize) {}
    fn on_org_started(&self, _: &str) {}
    fn on_org_complete(&self, _: &str, _: usize) {}
    fn on_personal_repos_started(&self) {}
    fn on_personal_repos_complete(&self, _: usize) {}
    fn on_error(&self, _: &str) {}
}

/// The core trait that all providers must implement.
///
/// This trait defines the interface for interacting with Git hosting providers
/// like GitHub, GitLab, and Bitbucket.
#[async_trait]
pub trait Provider: Send + Sync {
    /// Returns the provider kind (GitHub, GitLab, etc.).
    fn kind(&self) -> ProviderKind;

    /// Returns the display name for this provider instance.
    fn display_name(&self) -> &str;

    /// Validates that the credentials are valid.
    async fn validate_credentials(&self) -> Result<(), ProviderError>;

    /// Gets the authenticated user's username.
    async fn get_username(&self) -> Result<String, ProviderError>;

    /// Fetches all organizations the user belongs to.
    async fn get_organizations(&self) -> Result<Vec<Org>, ProviderError>;

    /// Fetches all repositories for a specific organization.
    async fn get_org_repos(&self, org: &str) -> Result<Vec<Repo>, ProviderError>;

    /// Fetches the user's personal repositories (not org repos).
    async fn get_user_repos(&self) -> Result<Vec<Repo>, ProviderError>;

    /// Returns current rate limit information.
    async fn get_rate_limit(&self) -> Result<RateLimitInfo, ProviderError>;

    /// High-level discovery that returns all repos with filtering.
    async fn discover_repos(
        &self,
        options: &DiscoveryOptions,
        progress: &dyn DiscoveryProgress,
    ) -> Result<Vec<OwnedRepo>, ProviderError>;

    /// Returns the clone URL for a repo (SSH or HTTPS based on preference).
    fn get_clone_url(&self, repo: &Repo, prefer_ssh: bool) -> String;

    /// Classifies whether the given account name is a personal user or an
    /// organization. Used by the Finder badge monitor to pick between "U" and
    /// "O" badges on workspace folders.
    async fn get_owner_type(&self, name: &str) -> Result<OwnerType, ProviderError>;
}

#[cfg(test)]
#[path = "traits_tests.rs"]
mod tests;
