//! Provider-specific configuration.
//!
//! Defines how individual Git hosting providers are configured,
//! including authentication methods and API endpoints.

use crate::types::ProviderKind;
use serde::{Deserialize, Serialize};

/// How to authenticate with a provider.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum AuthMethod {
    /// Use GitHub CLI (`gh auth token`)
    #[default]
    GhCli,
    /// Use environment variable
    Env,
    /// Use token directly from config (not recommended)
    Token,
}

/// Configuration for a single Git hosting provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderEntry {
    /// The type of provider (github, gitlab, etc.)
    #[serde(default)]
    pub kind: ProviderKind,

    /// Display name for this provider instance
    #[serde(default)]
    pub name: Option<String>,

    /// API base URL (required for GitHub Enterprise, optional for others)
    #[serde(default)]
    pub api_url: Option<String>,

    /// How to authenticate
    #[serde(default)]
    pub auth: AuthMethod,

    /// Environment variable name for token (when auth = "env")
    #[serde(default)]
    pub token_env: Option<String>,

    /// Token value (when auth = "token", not recommended)
    #[serde(default)]
    pub token: Option<String>,

    /// Whether to prefer SSH for cloning (default: true)
    #[serde(default = "default_true")]
    pub prefer_ssh: bool,

    /// Base directory override for this provider's repos
    #[serde(default)]
    pub base_path: Option<String>,

    /// Whether this provider is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

impl Default for ProviderEntry {
    fn default() -> Self {
        Self {
            kind: ProviderKind::GitHub,
            name: None,
            api_url: None,
            auth: AuthMethod::GhCli,
            token_env: None,
            token: None,
            prefer_ssh: true,
            base_path: None,
            enabled: true,
        }
    }
}

impl ProviderEntry {
    /// Creates a default GitHub.com provider entry.
    pub fn github() -> Self {
        Self {
            kind: ProviderKind::GitHub,
            name: Some("GitHub".to_string()),
            ..Default::default()
        }
    }

    /// Creates a GitHub Enterprise provider entry.
    pub fn github_enterprise(api_url: impl Into<String>, token_env: impl Into<String>) -> Self {
        Self {
            kind: ProviderKind::GitHubEnterprise,
            name: Some("GitHub Enterprise".to_string()),
            api_url: Some(api_url.into()),
            auth: AuthMethod::Env,
            token_env: Some(token_env.into()),
            ..Default::default()
        }
    }

    /// Returns the effective API URL for this provider.
    pub fn effective_api_url(&self) -> String {
        self.api_url
            .clone()
            .unwrap_or_else(|| self.kind.default_api_url().to_string())
    }

    /// Returns the display name for this provider.
    pub fn display_name(&self) -> String {
        self.name
            .clone()
            .unwrap_or_else(|| self.kind.display_name().to_string())
    }

    /// Returns the environment variable name for the token.
    pub fn effective_token_env(&self) -> Option<&str> {
        match self.auth {
            AuthMethod::Env => self.token_env.as_deref().or(Some("GITHUB_TOKEN")),
            _ => None,
        }
    }

    /// Validates the provider configuration.
    pub fn validate(&self) -> Result<(), String> {
        // GitHub Enterprise requires api_url
        if self.kind == ProviderKind::GitHubEnterprise && self.api_url.is_none() {
            return Err("GitHub Enterprise requires an api_url".to_string());
        }

        // Env auth requires token_env
        if self.auth == AuthMethod::Env && self.token_env.is_none() {
            return Err("Environment auth requires token_env to be set".to_string());
        }

        // Token auth requires token
        if self.auth == AuthMethod::Token && self.token.is_none() {
            return Err("Token auth requires token to be set".to_string());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_provider_entry() {
        let entry = ProviderEntry::default();
        assert_eq!(entry.kind, ProviderKind::GitHub);
        assert_eq!(entry.auth, AuthMethod::GhCli);
        assert!(entry.prefer_ssh);
        assert!(entry.enabled);
    }

    #[test]
    fn test_github_factory() {
        let entry = ProviderEntry::github();
        assert_eq!(entry.kind, ProviderKind::GitHub);
        assert_eq!(entry.display_name(), "GitHub");
    }

    #[test]
    fn test_github_enterprise_factory() {
        let entry = ProviderEntry::github_enterprise(
            "https://github.company.com/api/v3",
            "COMPANY_GITHUB_TOKEN",
        );
        assert_eq!(entry.kind, ProviderKind::GitHubEnterprise);
        assert_eq!(entry.auth, AuthMethod::Env);
        assert_eq!(entry.token_env, Some("COMPANY_GITHUB_TOKEN".to_string()));
    }

    #[test]
    fn test_effective_api_url_with_override() {
        let mut entry = ProviderEntry::github();
        entry.api_url = Some("https://custom-api.example.com".to_string());
        assert_eq!(entry.effective_api_url(), "https://custom-api.example.com");
    }

    #[test]
    fn test_effective_api_url_default() {
        let entry = ProviderEntry::github();
        assert_eq!(entry.effective_api_url(), "https://api.github.com");
    }

    #[test]
    fn test_validate_github_enterprise_without_url() {
        let entry = ProviderEntry {
            kind: ProviderKind::GitHubEnterprise,
            api_url: None,
            ..Default::default()
        };
        let result = entry.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("api_url"));
    }

    #[test]
    fn test_validate_env_auth_without_token_env() {
        let entry = ProviderEntry {
            auth: AuthMethod::Env,
            token_env: None,
            ..Default::default()
        };
        let result = entry.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("token_env"));
    }

    #[test]
    fn test_validate_token_auth_without_token() {
        let entry = ProviderEntry {
            auth: AuthMethod::Token,
            token: None,
            ..Default::default()
        };
        let result = entry.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("token"));
    }

    #[test]
    fn test_validate_valid_config() {
        let entry = ProviderEntry::github();
        assert!(entry.validate().is_ok());

        let entry = ProviderEntry {
            auth: AuthMethod::Env,
            token_env: Some("MY_TOKEN".to_string()),
            ..Default::default()
        };
        assert!(entry.validate().is_ok());
    }

    #[test]
    fn test_serde_roundtrip() {
        let entry = ProviderEntry {
            kind: ProviderKind::GitHub,
            name: Some("My GitHub".to_string()),
            auth: AuthMethod::Env,
            token_env: Some("MY_TOKEN".to_string()),
            prefer_ssh: false,
            ..Default::default()
        };

        let toml = toml::to_string(&entry).unwrap();
        let parsed: ProviderEntry = toml::from_str(&toml).unwrap();

        assert_eq!(parsed.kind, entry.kind);
        assert_eq!(parsed.name, entry.name);
        assert_eq!(parsed.auth, entry.auth);
        assert_eq!(parsed.token_env, entry.token_env);
        assert_eq!(parsed.prefer_ssh, entry.prefer_ssh);
    }

    #[test]
    fn test_auth_method_serde() {
        assert_eq!(
            serde_json::to_string(&AuthMethod::GhCli).unwrap(),
            "\"gh-cli\""
        );
        assert_eq!(serde_json::to_string(&AuthMethod::Env).unwrap(), "\"env\"");
        assert_eq!(
            serde_json::to_string(&AuthMethod::Token).unwrap(),
            "\"token\""
        );
    }
}
