//! Workspace configuration.
//!
//! Each workspace represents a sync target folder with its own provider,
//! selected organizations, and repository filters. Workspace config lives
//! inside the sync folder itself at `<root>/.git-same/config.toml`, making
//! workspaces portable and self-describing.

use super::{ConfigCloneOptions, FilterOptions, SyncMode};
use crate::types::ProviderKind;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Provider configuration scoped to a single workspace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceProvider {
    /// The type of provider (github, gitlab, etc.)
    #[serde(default)]
    pub kind: ProviderKind,

    /// API base URL (required for GitHub Enterprise)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_url: Option<String>,

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
            api_url: None,
            prefer_ssh: true,
        }
    }
}

impl WorkspaceProvider {
    /// Returns the effective API URL for this provider.
    pub fn effective_api_url(&self) -> String {
        self.api_url
            .clone()
            .unwrap_or_else(|| self.kind.default_api_url().to_string())
    }

    /// Returns the display name for this provider.
    pub fn display_name(&self) -> &str {
        self.kind.display_name()
    }
}

/// Configuration for a single workspace (sync target folder).
///
/// Stored at `<root>/.git-same/config.toml`. The `root_path` field is not
/// serialized — it is populated at load time from the `.git-same/` parent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    /// Absolute path to the workspace root (parent of `.git-same/`).
    ///
    /// Not stored in config.toml — derived from the file's location at load time.
    #[serde(skip)]
    pub root_path: PathBuf,

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
    /// Create a new workspace config for the given root directory.
    pub fn new_from_root(root: &Path) -> Self {
        Self {
            root_path: root.to_path_buf(),
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

    /// Returns the workspace root path (equivalent of old `expanded_base_path()`).
    pub fn expanded_base_path(&self) -> PathBuf {
        self.root_path.clone()
    }

    /// Returns a user-friendly label: `"~/repos (GitHub)"`.
    pub fn display_label(&self) -> String {
        let path_str = tilde_collapse_path(&self.root_path);
        format!("{} ({})", path_str, self.provider.kind.display_name())
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

/// Collapse the home directory prefix to `~` for display.
pub fn tilde_collapse_path(path: &Path) -> String {
    let s = path.to_string_lossy();
    if let Ok(home) = std::env::var("HOME") {
        if s.starts_with(&home) {
            return format!("~{}", &s[home.len()..]);
        }
    }
    s.to_string()
}

#[cfg(test)]
#[path = "workspace_tests.rs"]
mod tests;
