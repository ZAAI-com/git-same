//! Provider type definitions.
//!
//! Defines the supported Git hosting providers and their identifiers.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Identifies which Git hosting provider a repository belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum ProviderKind {
    /// GitHub.com (public)
    #[serde(rename = "github")]
    #[default]
    GitHub,
    /// GitHub Enterprise (Cloud & Server)
    #[serde(rename = "github-enterprise")]
    GitHubEnterprise,
    /// GitLab.com
    #[serde(rename = "gitlab")]
    GitLab,
    /// GitLab Dedicated & Self-Managed
    #[serde(rename = "gitlab-self-managed")]
    GitLabSelfManaged,
    /// Codeberg.org
    #[serde(rename = "codeberg")]
    Codeberg,
    /// Bitbucket.org
    #[serde(rename = "bitbucket")]
    Bitbucket,
}

impl ProviderKind {
    /// Returns a stable slug for path templating and cache keys.
    pub fn slug(&self) -> &'static str {
        match self {
            ProviderKind::GitHub => "github",
            ProviderKind::GitHubEnterprise => "github-enterprise",
            ProviderKind::GitLab => "gitlab",
            ProviderKind::GitLabSelfManaged => "gitlab-self-managed",
            ProviderKind::Codeberg => "codeberg",
            ProviderKind::Bitbucket => "bitbucket",
        }
    }

    /// Returns the default API base URL for this provider.
    pub fn default_api_url(&self) -> &'static str {
        match self {
            ProviderKind::GitHub => "https://api.github.com",
            ProviderKind::GitHubEnterprise => "", // Must be configured
            ProviderKind::GitLab => "https://gitlab.com/api/v4",
            ProviderKind::GitLabSelfManaged => "", // Must be configured
            ProviderKind::Codeberg => "https://codeberg.org/api/v1",
            ProviderKind::Bitbucket => "https://api.bitbucket.org/2.0",
        }
    }

    /// Returns the default git host for SSH URLs.
    pub fn default_ssh_host(&self) -> &'static str {
        match self {
            ProviderKind::GitHub => "github.com",
            ProviderKind::GitHubEnterprise => "", // Must be configured
            ProviderKind::GitLab => "gitlab.com",
            ProviderKind::GitLabSelfManaged => "", // Must be configured
            ProviderKind::Codeberg => "codeberg.org",
            ProviderKind::Bitbucket => "bitbucket.org",
        }
    }

    /// Returns true if this provider requires custom URL configuration.
    pub fn requires_custom_url(&self) -> bool {
        matches!(
            self,
            ProviderKind::GitHubEnterprise | ProviderKind::GitLabSelfManaged
        )
    }

    /// Returns the human-readable name for this provider.
    pub fn display_name(&self) -> &'static str {
        match self {
            ProviderKind::GitHub => "GitHub",
            ProviderKind::GitHubEnterprise => "GitHub Enterprise",
            ProviderKind::GitLab => "GitLab",
            ProviderKind::GitLabSelfManaged => "GitLab Self-Managed",
            ProviderKind::Codeberg => "Codeberg",
            ProviderKind::Bitbucket => "Bitbucket",
        }
    }

    /// Returns all supported provider kinds.
    pub fn all() -> &'static [ProviderKind] {
        &[
            ProviderKind::GitHub,
            ProviderKind::GitHubEnterprise,
            ProviderKind::GitLab,
            ProviderKind::GitLabSelfManaged,
            ProviderKind::Codeberg,
            ProviderKind::Bitbucket,
        ]
    }
}

impl fmt::Display for ProviderKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

impl std::str::FromStr for ProviderKind {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "github" | "gh" => Ok(ProviderKind::GitHub),
            "github-enterprise" | "ghe" | "github_enterprise" => Ok(ProviderKind::GitHubEnterprise),
            "gitlab" | "gl" => Ok(ProviderKind::GitLab),
            "gitlab-self-managed" | "glsm" | "gitlab_self_managed" => {
                Ok(ProviderKind::GitLabSelfManaged)
            }
            "codeberg" | "cb" => Ok(ProviderKind::Codeberg),
            "bitbucket" | "bb" => Ok(ProviderKind::Bitbucket),
            _ => Err(format!(
                "Unknown provider: '{}'. Supported: github, github-enterprise, gitlab, \
                 gitlab-self-managed, codeberg, bitbucket",
                s
            )),
        }
    }
}

#[cfg(test)]
#[path = "provider_tests.rs"]
mod tests;
