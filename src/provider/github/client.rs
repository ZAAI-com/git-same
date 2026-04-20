//! GitHub API client implementation.

use async_trait::async_trait;
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION, USER_AGENT};
use reqwest::Client;
use tracing::{debug, trace};

use super::pagination::fetch_all_pages;
use super::GITHUB_API_URL;
use crate::errors::ProviderError;
use crate::provider::traits::*;
use crate::types::{Org, OwnedRepo, OwnerType, ProviderKind, Repo};

/// Default timeout for API requests in seconds.
const DEFAULT_TIMEOUT_SECS: u64 = 60;

/// GitHub provider implementation.
///
/// Supports both github.com and GitHub Enterprise Server.
pub struct GitHubProvider {
    /// HTTP client
    client: Client,
    /// Authentication credentials
    credentials: Credentials,
    /// Display name for this provider instance
    display_name: String,
}

impl GitHubProvider {
    /// Creates a new GitHub provider with default timeout.
    pub fn new(
        credentials: Credentials,
        display_name: impl Into<String>,
    ) -> Result<Self, ProviderError> {
        Self::with_timeout(credentials, display_name, DEFAULT_TIMEOUT_SECS)
    }

    /// Creates a new GitHub provider with custom timeout.
    pub fn with_timeout(
        credentials: Credentials,
        display_name: impl Into<String>,
        timeout_secs: u64,
    ) -> Result<Self, ProviderError> {
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, HeaderValue::from_static("gisa-cli/0.1.0"));
        headers.insert(
            ACCEPT,
            HeaderValue::from_static("application/vnd.github+json"),
        );
        headers.insert(
            "X-GitHub-Api-Version",
            HeaderValue::from_static("2022-11-28"),
        );

        let client = Client::builder()
            .default_headers(headers)
            .timeout(std::time::Duration::from_secs(timeout_secs))
            .build()
            .map_err(|e| ProviderError::Configuration(e.to_string()))?;

        Ok(Self {
            client,
            credentials,
            display_name: display_name.into(),
        })
    }

    /// Constructs a full API URL from a path.
    fn api_url(&self, path: &str) -> String {
        format!("{}{}", self.credentials.api_base_url, path)
    }

    /// Makes an authenticated GET request.
    async fn get<T: serde::de::DeserializeOwned>(&self, url: &str) -> Result<T, ProviderError> {
        trace!(url, "Making authenticated GET request");

        let response = self
            .client
            .get(url)
            .header(AUTHORIZATION, format!("Bearer {}", self.credentials.token))
            .send()
            .await
            .map_err(|e| ProviderError::Network(e.to_string()))?;

        let status = response.status();
        trace!(url, status = %status, "Received response");

        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            debug!(url, status = %status, "API request failed");
            return Err(ProviderError::from_status(status.as_u16(), body));
        }

        response
            .json()
            .await
            .map_err(|e| ProviderError::Parse(e.to_string()))
    }

    /// Fetches all pages from an endpoint.
    async fn get_paginated<T: serde::de::DeserializeOwned>(
        &self,
        url: &str,
    ) -> Result<Vec<T>, ProviderError> {
        fetch_all_pages(&self.client, &self.credentials.token, url).await
    }

    /// Determines if this is GitHub.com or GitHub Enterprise.
    fn is_github_com(&self) -> bool {
        self.credentials.api_base_url == GITHUB_API_URL
    }
}

#[async_trait]
impl Provider for GitHubProvider {
    fn kind(&self) -> ProviderKind {
        if self.is_github_com() {
            ProviderKind::GitHub
        } else {
            ProviderKind::GitHubEnterprise
        }
    }

    fn display_name(&self) -> &str {
        &self.display_name
    }

    async fn validate_credentials(&self) -> Result<(), ProviderError> {
        // Make a simple API call to verify the token works
        self.get_username().await?;
        Ok(())
    }

    async fn get_username(&self) -> Result<String, ProviderError> {
        #[derive(serde::Deserialize)]
        struct User {
            login: String,
        }

        let url = self.api_url("/user");
        let user: User = self.get(&url).await?;
        Ok(user.login)
    }

    async fn get_organizations(&self) -> Result<Vec<Org>, ProviderError> {
        let url = self.api_url("/user/orgs");
        self.get_paginated(&url).await
    }

    async fn get_org_repos(&self, org: &str) -> Result<Vec<Repo>, ProviderError> {
        let url = self.api_url(&format!("/orgs/{}/repos", org));
        self.get_paginated(&url).await
    }

    async fn get_user_repos(&self) -> Result<Vec<Repo>, ProviderError> {
        let url = self.api_url("/user/repos?affiliation=owner");
        self.get_paginated(&url).await
    }

    async fn get_rate_limit(&self) -> Result<RateLimitInfo, ProviderError> {
        #[derive(serde::Deserialize)]
        struct RateLimitResponse {
            rate: RateInfo,
        }

        #[derive(serde::Deserialize)]
        struct RateInfo {
            limit: u32,
            remaining: u32,
            reset: i64,
        }

        let url = self.api_url("/rate_limit");
        let response: RateLimitResponse = self.get(&url).await?;

        Ok(RateLimitInfo {
            limit: response.rate.limit,
            remaining: response.rate.remaining,
            reset_at: Some(response.rate.reset),
        })
    }

    async fn discover_repos(
        &self,
        options: &DiscoveryOptions,
        progress: &dyn DiscoveryProgress,
    ) -> Result<Vec<OwnedRepo>, ProviderError> {
        debug!(provider = %self.display_name, "Starting repository discovery");

        let username = self.get_username().await?;
        debug!(username, "Authenticated user");

        let mut all_repos = Vec::new();

        // Get organizations
        let orgs = self.get_organizations().await?;
        let orgs_count = orgs.len();
        let filtered_orgs: Vec<_> = orgs
            .into_iter()
            .filter(|o| options.should_include_org(&o.login))
            .collect();

        debug!(
            total_orgs = orgs_count,
            filtered_orgs = filtered_orgs.len(),
            "Discovered organizations"
        );
        progress.on_orgs_discovered(filtered_orgs.len());

        // Fetch repos for each org
        for org in &filtered_orgs {
            progress.on_org_started(&org.login);

            match self.get_org_repos(&org.login).await {
                Ok(repos) => {
                    let filtered: Vec<_> = repos
                        .into_iter()
                        .filter(|r| options.should_include(r))
                        .collect();

                    let count = filtered.len();
                    for repo in filtered {
                        all_repos.push(OwnedRepo::new(&org.login, repo));
                    }

                    progress.on_org_complete(&org.login, count);
                }
                Err(e) => {
                    progress.on_error(&format!("Error fetching repos for {}: {}", org.login, e));
                    progress.on_org_complete(&org.login, 0);
                }
            }
        }

        // Fetch personal repos
        progress.on_personal_repos_started();

        match self.get_user_repos().await {
            Ok(repos) => {
                let filtered: Vec<_> = repos
                    .into_iter()
                    // Skip repos already added via org
                    .filter(|r| !all_repos.iter().any(|or| or.repo.id == r.id))
                    .filter(|r| options.should_include(r))
                    .collect();

                let count = filtered.len();
                for repo in filtered {
                    all_repos.push(OwnedRepo::new(&username, repo));
                }

                progress.on_personal_repos_complete(count);
            }
            Err(e) => {
                progress.on_error(&format!("Error fetching personal repos: {}", e));
                progress.on_personal_repos_complete(0);
            }
        }

        Ok(all_repos)
    }

    fn get_clone_url(&self, repo: &Repo, prefer_ssh: bool) -> String {
        if prefer_ssh {
            repo.ssh_url.clone()
        } else {
            repo.clone_url.clone()
        }
    }

    async fn get_owner_type(&self, name: &str) -> Result<OwnerType, ProviderError> {
        #[derive(serde::Deserialize)]
        struct UserOrOrg {
            #[serde(rename = "type")]
            kind: String,
        }

        let url = self.api_url(&format!("/users/{}", name));
        let payload: UserOrOrg = self.get(&url).await?;
        Ok(match payload.kind.as_str() {
            "User" => OwnerType::User,
            "Organization" => OwnerType::Organization,
            _ => OwnerType::Unknown,
        })
    }
}

#[cfg(test)]
#[path = "client_tests.rs"]
mod tests;
