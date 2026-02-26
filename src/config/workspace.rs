//! Workspace configuration.
//!
//! Each workspace represents a sync target folder with its own provider,
//! selected organizations, and repository filters. Each workspace is a
//! subdirectory of `~/.config/git-same/<name>/` containing `workspace-config.toml`.

use super::provider_config::AuthMethod;
use super::{ConfigCloneOptions, FilterOptions, SyncMode};
use crate::types::ProviderKind;
use serde::{Deserialize, Serialize};

/// Provider configuration scoped to a single workspace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceProvider {
    /// The type of provider (github, gitlab, etc.)
    #[serde(default)]
    pub kind: ProviderKind,

    /// How to authenticate
    #[serde(default)]
    pub auth: AuthMethod,

    /// API base URL (required for GitHub Enterprise)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_url: Option<String>,

    /// Environment variable name for token (when auth = "env")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token_env: Option<String>,

    /// Whether to prefer SSH for cloning (default: true)
    #[serde(default = "default_true")]
    pub prefer_ssh: bool,
}

fn default_true() -> bool {
    true
}

impl Default for WorkspaceProvider {
    fn default() -> Self {
        Self {
            kind: ProviderKind::GitHub,
            auth: AuthMethod::GhCli,
            api_url: None,
            token_env: None,
            prefer_ssh: true,
        }
    }
}

impl WorkspaceProvider {
    /// Convert to a `ProviderEntry` for use with existing provider/auth infrastructure.
    pub fn to_provider_entry(&self) -> super::ProviderEntry {
        super::ProviderEntry {
            kind: self.kind,
            name: Some(self.kind.display_name().to_string()),
            api_url: self.api_url.clone(),
            auth: self.auth.clone(),
            token_env: self.token_env.clone(),
            token: None,
            prefer_ssh: self.prefer_ssh,
            base_path: None,
            enabled: true,
        }
    }
}

/// Configuration for a single workspace (sync target folder).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    /// Workspace name, derived from the config folder name at load time.
    ///
    /// Not stored in `workspace-config.toml` — the folder name is the source of truth.
    #[serde(skip_serializing, default)]
    pub name: String,

    /// Absolute path to the folder where repos are cloned.
    pub base_path: String,

    /// Provider configuration for this workspace.
    pub provider: WorkspaceProvider,

    /// The authenticated username (discovered during setup).
    #[serde(default)]
    pub username: String,

    /// Selected organizations to sync (empty = all).
    #[serde(default)]
    pub orgs: Vec<String>,

    /// Specific repos to include (empty = all from selected orgs).
    #[serde(default)]
    pub include_repos: Vec<String>,

    /// Repos to exclude by full name (e.g., "org/repo").
    #[serde(default)]
    pub exclude_repos: Vec<String>,

    /// Directory structure pattern override (None = use global default).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub structure: Option<String>,

    /// Sync mode override (None = use global default).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sync_mode: Option<SyncMode>,

    /// Clone options override (None = use global default).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(rename = "clone")]
    pub clone_options: Option<ConfigCloneOptions>,

    /// Filter options.
    #[serde(default)]
    pub filters: FilterOptions,

    /// Concurrency override (None = use global default).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub concurrency: Option<usize>,

    /// Dashboard auto-refresh interval override in seconds (None = use global default).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refresh_interval: Option<u64>,

    /// ISO 8601 timestamp of last sync.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_synced: Option<String>,
}

impl WorkspaceConfig {
    /// Create a new workspace config with minimal required fields.
    pub fn new(name: impl Into<String>, base_path: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            base_path: base_path.into(),
            provider: WorkspaceProvider::default(),
            username: String::new(),
            orgs: Vec::new(),
            include_repos: Vec::new(),
            exclude_repos: Vec::new(),
            structure: None,
            sync_mode: None,
            clone_options: None,
            filters: FilterOptions::default(),
            concurrency: None,
            refresh_interval: None,
            last_synced: None,
        }
    }

    /// Expand ~ in base_path to the actual home directory.
    pub fn expanded_base_path(&self) -> std::path::PathBuf {
        let expanded = shellexpand::tilde(&self.base_path);
        std::path::PathBuf::from(expanded.as_ref())
    }

    /// Returns a user-friendly label: `"~/repos (GitHub)"`.
    ///
    /// This is the primary user-facing workspace identity. The internal `name`
    /// field is a filesystem key and should never be shown to users.
    pub fn display_label(&self) -> String {
        format!("{} ({})", self.base_path, self.provider.kind.display_name())
    }

    /// Returns a short display summary for selectors.
    pub fn summary(&self) -> String {
        let orgs = if self.orgs.is_empty() {
            "all orgs".to_string()
        } else {
            format!("{} org(s)", self.orgs.len())
        };
        let synced = self.last_synced.as_deref().unwrap_or("never synced");
        format!("{} ({}, {})", self.display_label(), orgs, synced)
    }

    /// Serialize to TOML string.
    pub fn to_toml(&self) -> Result<String, crate::errors::AppError> {
        toml::to_string_pretty(self).map_err(|e| {
            crate::errors::AppError::config(format!("Failed to serialize workspace config: {}", e))
        })
    }

    /// Parse from TOML string.
    pub fn from_toml(content: &str) -> Result<Self, crate::errors::AppError> {
        toml::from_str(content).map_err(|e| {
            crate::errors::AppError::config(format!("Failed to parse workspace config: {}", e))
        })
    }
}

#[cfg(test)]
#[path = "workspace_tests.rs"]
mod tests;
