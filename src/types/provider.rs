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
    /// GitHub Enterprise Server (self-hosted)
    #[serde(rename = "github-enterprise")]
    GitHubEnterprise,
    /// GitLab.com or self-hosted GitLab
    #[serde(rename = "gitlab")]
    GitLab,
    /// Atlassian Bitbucket
    #[serde(rename = "bitbucket")]
    Bitbucket,
}

impl ProviderKind {
    /// Returns the default API base URL for this provider.
    pub fn default_api_url(&self) -> &'static str {
        match self {
            ProviderKind::GitHub => "https://api.github.com",
            ProviderKind::GitHubEnterprise => "", // Must be configured
            ProviderKind::GitLab => "https://gitlab.com/api/v4",
            ProviderKind::Bitbucket => "https://api.bitbucket.org/2.0",
        }
    }

    /// Returns the default git host for SSH URLs.
    pub fn default_ssh_host(&self) -> &'static str {
        match self {
            ProviderKind::GitHub => "github.com",
            ProviderKind::GitHubEnterprise => "", // Must be configured
            ProviderKind::GitLab => "gitlab.com",
            ProviderKind::Bitbucket => "bitbucket.org",
        }
    }

    /// Returns true if this provider requires custom URL configuration.
    pub fn requires_custom_url(&self) -> bool {
        matches!(self, ProviderKind::GitHubEnterprise)
    }

    /// Returns the human-readable name for this provider.
    pub fn display_name(&self) -> &'static str {
        match self {
            ProviderKind::GitHub => "GitHub",
            ProviderKind::GitHubEnterprise => "GitHub Enterprise",
            ProviderKind::GitLab => "GitLab",
            ProviderKind::Bitbucket => "Bitbucket",
        }
    }

    /// Returns all supported provider kinds.
    pub fn all() -> &'static [ProviderKind] {
        &[
            ProviderKind::GitHub,
            ProviderKind::GitHubEnterprise,
            ProviderKind::GitLab,
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
            "bitbucket" | "bb" => Ok(ProviderKind::Bitbucket),
            _ => Err(format!(
                "Unknown provider: '{}'. Supported: github, github-enterprise, gitlab, bitbucket",
                s
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_is_github() {
        assert_eq!(ProviderKind::default(), ProviderKind::GitHub);
    }

    #[test]
    fn test_display() {
        assert_eq!(format!("{}", ProviderKind::GitHub), "GitHub");
        assert_eq!(
            format!("{}", ProviderKind::GitHubEnterprise),
            "GitHub Enterprise"
        );
        assert_eq!(format!("{}", ProviderKind::GitLab), "GitLab");
        assert_eq!(format!("{}", ProviderKind::Bitbucket), "Bitbucket");
    }

    #[test]
    fn test_from_str() {
        assert_eq!(
            "github".parse::<ProviderKind>().unwrap(),
            ProviderKind::GitHub
        );
        assert_eq!("gh".parse::<ProviderKind>().unwrap(), ProviderKind::GitHub);
        assert_eq!(
            "GITHUB".parse::<ProviderKind>().unwrap(),
            ProviderKind::GitHub
        );

        assert_eq!(
            "github-enterprise".parse::<ProviderKind>().unwrap(),
            ProviderKind::GitHubEnterprise
        );
        assert_eq!(
            "ghe".parse::<ProviderKind>().unwrap(),
            ProviderKind::GitHubEnterprise
        );

        assert_eq!(
            "gitlab".parse::<ProviderKind>().unwrap(),
            ProviderKind::GitLab
        );
        assert_eq!("gl".parse::<ProviderKind>().unwrap(), ProviderKind::GitLab);

        assert_eq!(
            "bitbucket".parse::<ProviderKind>().unwrap(),
            ProviderKind::Bitbucket
        );
        assert_eq!(
            "bb".parse::<ProviderKind>().unwrap(),
            ProviderKind::Bitbucket
        );
    }

    #[test]
    fn test_from_str_invalid() {
        let result = "invalid".parse::<ProviderKind>();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unknown provider"));
    }

    #[test]
    fn test_default_api_urls() {
        assert_eq!(
            ProviderKind::GitHub.default_api_url(),
            "https://api.github.com"
        );
        assert_eq!(
            ProviderKind::GitLab.default_api_url(),
            "https://gitlab.com/api/v4"
        );
        assert_eq!(
            ProviderKind::Bitbucket.default_api_url(),
            "https://api.bitbucket.org/2.0"
        );
        // GitHub Enterprise has empty default (must be configured)
        assert_eq!(ProviderKind::GitHubEnterprise.default_api_url(), "");
    }

    #[test]
    fn test_requires_custom_url() {
        assert!(!ProviderKind::GitHub.requires_custom_url());
        assert!(ProviderKind::GitHubEnterprise.requires_custom_url());
        assert!(!ProviderKind::GitLab.requires_custom_url());
        assert!(!ProviderKind::Bitbucket.requires_custom_url());
    }

    #[test]
    fn test_serde_serialization() {
        let json = serde_json::to_string(&ProviderKind::GitHub).unwrap();
        assert_eq!(json, "\"github\"");

        let json = serde_json::to_string(&ProviderKind::GitHubEnterprise).unwrap();
        assert_eq!(json, "\"github-enterprise\"");
    }

    #[test]
    fn test_serde_deserialization() {
        let kind: ProviderKind = serde_json::from_str("\"github\"").unwrap();
        assert_eq!(kind, ProviderKind::GitHub);

        let kind: ProviderKind = serde_json::from_str("\"gitlab\"").unwrap();
        assert_eq!(kind, ProviderKind::GitLab);
    }

    #[test]
    fn test_all_providers() {
        let all = ProviderKind::all();
        assert_eq!(all.len(), 4);
        assert!(all.contains(&ProviderKind::GitHub));
        assert!(all.contains(&ProviderKind::GitHubEnterprise));
        assert!(all.contains(&ProviderKind::GitLab));
        assert!(all.contains(&ProviderKind::Bitbucket));
    }

    #[test]
    fn test_equality_and_hash() {
        use std::collections::HashSet;

        let mut set = HashSet::new();
        set.insert(ProviderKind::GitHub);
        set.insert(ProviderKind::GitHub); // Duplicate

        assert_eq!(set.len(), 1);
        assert!(set.contains(&ProviderKind::GitHub));
    }
}
