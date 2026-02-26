//! Provider-specific configuration.
//!
//! Defines how individual Git hosting providers are configured,
//! including authentication and API endpoints.

use crate::types::ProviderKind;
use serde::{Deserialize, Serialize};

/// How to authenticate with a provider.
///
/// Currently only GitHub CLI is supported.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum AuthMethod {
    /// Use GitHub CLI (`gh auth token`)
    #[default]
    GhCli,
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

    /// Validates the provider configuration.
    pub fn validate(&self) -> Result<(), String> {
        Ok(())
    }
}

#[cfg(test)]
#[path = "provider_config_tests.rs"]
mod tests;
