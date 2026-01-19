//! Provider trait definitions.
//!
//! The [`Provider`] trait defines the interface that all Git hosting
//! provider implementations must implement.

use async_trait::async_trait;

use crate::errors::ProviderError;
use crate::types::{Org, OwnedRepo, ProviderKind, Repo};

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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_credentials_builder() {
        let creds = Credentials::new("token123", "https://api.github.com")
            .with_username("testuser");

        assert_eq!(creds.token, "token123");
        assert_eq!(creds.api_base_url, "https://api.github.com");
        assert_eq!(creds.username, Some("testuser".to_string()));
    }

    #[test]
    fn test_rate_limit_exhausted() {
        let info = RateLimitInfo {
            limit: 5000,
            remaining: 0,
            reset_at: None,
        };
        assert!(info.is_exhausted());

        let info = RateLimitInfo {
            limit: 5000,
            remaining: 100,
            reset_at: None,
        };
        assert!(!info.is_exhausted());
    }

    #[test]
    fn test_discovery_options_builder() {
        let options = DiscoveryOptions::new()
            .with_archived(true)
            .with_forks(true)
            .with_orgs(vec!["org1".to_string(), "org2".to_string()])
            .with_exclusions(vec!["org1/skip".to_string()]);

        assert!(options.include_archived);
        assert!(options.include_forks);
        assert_eq!(options.org_filter.len(), 2);
        assert_eq!(options.exclude_repos.len(), 1);
    }

    #[test]
    fn test_should_include_repo() {
        let options = DiscoveryOptions::new();

        // Non-archived, non-fork repo should be included
        let repo = Repo::test("repo", "org");
        assert!(options.should_include(&repo));
    }

    #[test]
    fn test_should_exclude_archived() {
        let options = DiscoveryOptions::new().with_archived(false);

        let mut repo = Repo::test("repo", "org");
        repo.archived = true;
        assert!(!options.should_include(&repo));

        let options = DiscoveryOptions::new().with_archived(true);
        assert!(options.should_include(&repo));
    }

    #[test]
    fn test_should_exclude_forks() {
        let options = DiscoveryOptions::new().with_forks(false);

        let mut repo = Repo::test("repo", "org");
        repo.fork = true;
        assert!(!options.should_include(&repo));

        let options = DiscoveryOptions::new().with_forks(true);
        assert!(options.should_include(&repo));
    }

    #[test]
    fn test_should_exclude_by_name() {
        let options =
            DiscoveryOptions::new().with_exclusions(vec!["org/excluded-repo".to_string()]);

        let mut repo = Repo::test("excluded-repo", "org");
        repo.full_name = "org/excluded-repo".to_string();
        assert!(!options.should_include(&repo));

        let mut repo = Repo::test("included-repo", "org");
        repo.full_name = "org/included-repo".to_string();
        assert!(options.should_include(&repo));
    }

    #[test]
    fn test_should_include_org_empty_filter() {
        let options = DiscoveryOptions::new();
        assert!(options.should_include_org("any-org"));
    }

    #[test]
    fn test_should_include_org_with_filter() {
        let options =
            DiscoveryOptions::new().with_orgs(vec!["allowed-org".to_string()]);

        assert!(options.should_include_org("allowed-org"));
        assert!(!options.should_include_org("other-org"));
    }

    #[test]
    fn test_no_progress_compiles() {
        let progress = NoProgress;
        progress.on_orgs_discovered(5);
        progress.on_org_started("test");
        progress.on_org_complete("test", 10);
        progress.on_personal_repos_started();
        progress.on_personal_repos_complete(3);
        progress.on_error("test error");
    }
}
