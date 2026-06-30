//! Mock provider for testing.
//!
//! This module provides a configurable mock implementation of the [`Provider`]
//! trait for use in unit tests.

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use super::traits::*;
use crate::errors::ProviderError;
use crate::types::{Org, OwnedRepo, OwnerType, ProviderKind, Repo};

/// A mock provider that can be configured with predefined responses.
pub struct MockProvider {
    /// The provider kind to report
    pub kind: ProviderKind,
    /// Display name
    pub display_name: String,
    /// The username to return
    pub username: String,
    /// Organizations to return
    pub orgs: Vec<Org>,
    /// Repos per organization
    pub org_repos: HashMap<String, Vec<Repo>>,
    /// Personal repos
    pub user_repos: Vec<Repo>,
    /// Rate limit info to return
    pub rate_limit: RateLimitInfo,
    /// Track method calls for assertions
    pub call_log: Arc<Mutex<Vec<String>>>,
    /// Should auth validation fail?
    pub should_fail_auth: bool,
    /// Should org fetching fail?
    pub should_fail_orgs: bool,
}

impl Default for MockProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl MockProvider {
    /// Creates a new mock provider with defaults.
    pub fn new() -> Self {
        Self {
            kind: ProviderKind::GitHub,
            display_name: "Mock GitHub".to_string(),
            username: "testuser".to_string(),
            orgs: vec![],
            org_repos: HashMap::new(),
            user_repos: vec![],
            rate_limit: RateLimitInfo {
                limit: 5000,
                remaining: 5000,
                reset_at: None,
            },
            call_log: Arc::new(Mutex::new(vec![])),
            should_fail_auth: false,
            should_fail_orgs: false,
        }
    }

    /// Sets the username.
    pub fn with_username(mut self, username: impl Into<String>) -> Self {
        self.username = username.into();
        self
    }

    /// Sets the organizations.
    pub fn with_orgs(mut self, orgs: Vec<Org>) -> Self {
        self.orgs = orgs;
        self
    }

    /// Sets repos for an organization.
    pub fn with_org_repos(mut self, org: impl Into<String>, repos: Vec<Repo>) -> Self {
        self.org_repos.insert(org.into(), repos);
        self
    }

    /// Sets personal repos.
    pub fn with_user_repos(mut self, repos: Vec<Repo>) -> Self {
        self.user_repos = repos;
        self
    }

    /// Makes auth validation fail.
    pub fn with_auth_failure(mut self) -> Self {
        self.should_fail_auth = true;
        self
    }

    /// Makes org fetching fail.
    pub fn with_orgs_failure(mut self) -> Self {
        self.should_fail_orgs = true;
        self
    }

    /// Records a method call.
    fn log_call(&self, method: &str) {
        let mut log = self.call_log.lock().unwrap();
        log.push(method.to_string());
    }

    /// Returns all recorded method calls.
    pub fn get_calls(&self) -> Vec<String> {
        self.call_log.lock().unwrap().clone()
    }

    /// Clears the call log.
    pub fn clear_calls(&self) {
        self.call_log.lock().unwrap().clear();
    }
}

#[async_trait]
impl Provider for MockProvider {
    fn kind(&self) -> ProviderKind {
        self.kind
    }

    fn display_name(&self) -> &str {
        &self.display_name
    }

    async fn validate_credentials(&self) -> Result<(), ProviderError> {
        self.log_call("validate_credentials");
        if self.should_fail_auth {
            Err(ProviderError::Authentication(
                "Mock authentication failure".to_string(),
            ))
        } else {
            Ok(())
        }
    }

    async fn get_username(&self) -> Result<String, ProviderError> {
        self.log_call("get_username");
        if self.should_fail_auth {
            Err(ProviderError::Authentication(
                "Mock authentication failure".to_string(),
            ))
        } else {
            Ok(self.username.clone())
        }
    }

    async fn get_organizations(&self) -> Result<Vec<Org>, ProviderError> {
        self.log_call("get_organizations");
        if self.should_fail_orgs {
            Err(ProviderError::Api {
                status: 500,
                message: "Mock server error".to_string(),
            })
        } else {
            Ok(self.orgs.clone())
        }
    }

    async fn get_org_repos(&self, org: &str) -> Result<Vec<Repo>, ProviderError> {
        self.log_call(&format!("get_org_repos:{}", org));
        Ok(self.org_repos.get(org).cloned().unwrap_or_default())
    }

    async fn get_user_repos(&self) -> Result<Vec<Repo>, ProviderError> {
        self.log_call("get_user_repos");
        Ok(self.user_repos.clone())
    }

    async fn get_rate_limit(&self) -> Result<RateLimitInfo, ProviderError> {
        self.log_call("get_rate_limit");
        Ok(self.rate_limit.clone())
    }

    async fn discover_repos(
        &self,
        options: &DiscoveryOptions,
        progress: &dyn DiscoveryProgress,
    ) -> Result<Vec<OwnedRepo>, ProviderError> {
        self.log_call("discover_repos");

        let mut repos = Vec::new();

        // Report orgs
        let filtered_orgs: Vec<_> = self
            .orgs
            .iter()
            .filter(|o| options.should_include_org(&o.login))
            .collect();

        progress.on_orgs_discovered(filtered_orgs.len());

        // Fetch org repos
        for org in filtered_orgs {
            progress.on_org_started(&org.login);
            let mut org_count = 0usize;

            if let Some(org_repos) = self.org_repos.get(&org.login) {
                let filtered: Vec<_> = org_repos
                    .iter()
                    .filter(|r| options.should_include(r))
                    .collect();

                for repo in filtered {
                    repos.push(OwnedRepo::new(&org.login, repo.clone()));
                    org_count += 1;
                }
            }

            progress.on_org_complete(&org.login, org_count);
        }

        // Fetch personal repos
        progress.on_personal_repos_started();

        let personal_filtered: Vec<_> = self
            .user_repos
            .iter()
            .filter(|r| options.should_include(r))
            .filter(|r| !repos.iter().any(|or| or.repo.id == r.id))
            .collect();

        let personal_count = personal_filtered.len();
        for repo in personal_filtered {
            repos.push(OwnedRepo::new(&self.username, repo.clone()));
        }

        progress.on_personal_repos_complete(personal_count);

        Ok(repos)
    }

    fn get_clone_url(&self, repo: &Repo, prefer_ssh: bool) -> String {
        if prefer_ssh {
            repo.ssh_url.clone()
        } else {
            repo.clone_url.clone()
        }
    }

    async fn get_owner_type(&self, _name: &str) -> Result<OwnerType, ProviderError> {
        Ok(OwnerType::Organization)
    }
}

#[cfg(test)]
#[path = "mock_tests.rs"]
mod tests;
