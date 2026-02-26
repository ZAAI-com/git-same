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
#[path = "provider_config_tests.rs"]
mod tests;
