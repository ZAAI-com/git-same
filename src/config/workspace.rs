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
            last_synced: None,
        }
    }

    /// Expand ~ in base_path to the actual home directory.
    pub fn expanded_base_path(&self) -> std::path::PathBuf {
        let expanded = shellexpand::tilde(&self.base_path);
        std::path::PathBuf::from(expanded.as_ref())
    }

    /// Returns a short display summary for selectors.
    pub fn summary(&self) -> String {
        let orgs = if self.orgs.is_empty() {
            "all orgs".to_string()
        } else {
            format!("{} org(s)", self.orgs.len())
        };
        let synced = self.last_synced.as_deref().unwrap_or("never synced");
        format!("{} — {} ({}, {})", self.name, self.base_path, orgs, synced)
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
mod tests {
    use super::*;

    #[test]
    fn test_new_workspace_config() {
        let ws = WorkspaceConfig::new("github", "~/github");
        assert_eq!(ws.name, "github");
        assert_eq!(ws.base_path, "~/github");
        assert_eq!(ws.provider.kind, ProviderKind::GitHub);
        assert!(ws.orgs.is_empty());
        assert!(ws.last_synced.is_none());
    }

    #[test]
    fn test_workspace_provider_default() {
        let provider = WorkspaceProvider::default();
        assert_eq!(provider.kind, ProviderKind::GitHub);
        assert_eq!(provider.auth, AuthMethod::GhCli);
        assert!(provider.prefer_ssh);
        assert!(provider.api_url.is_none());
    }

    #[test]
    fn test_workspace_provider_to_provider_entry() {
        let provider = WorkspaceProvider {
            kind: ProviderKind::GitHubEnterprise,
            auth: AuthMethod::Env,
            api_url: Some("https://github.corp.com/api/v3".to_string()),
            token_env: Some("CORP_TOKEN".to_string()),
            prefer_ssh: false,
        };
        let entry = provider.to_provider_entry();
        assert_eq!(entry.kind, ProviderKind::GitHubEnterprise);
        assert_eq!(entry.auth, AuthMethod::Env);
        assert_eq!(
            entry.api_url,
            Some("https://github.corp.com/api/v3".to_string())
        );
        assert_eq!(entry.token_env, Some("CORP_TOKEN".to_string()));
        assert!(!entry.prefer_ssh);
        assert!(entry.enabled);
    }

    #[test]
    fn test_serde_roundtrip() {
        let ws = WorkspaceConfig {
            name: "my-workspace".to_string(),
            base_path: "~/code/repos".to_string(),
            provider: WorkspaceProvider {
                kind: ProviderKind::GitHub,
                auth: AuthMethod::GhCli,
                api_url: None,
                token_env: None,
                prefer_ssh: true,
            },
            username: "testuser".to_string(),
            orgs: vec!["org1".to_string(), "org2".to_string()],
            include_repos: vec![],
            exclude_repos: vec!["org1/skip-this".to_string()],
            structure: Some("{org}/{repo}".to_string()),
            sync_mode: Some(SyncMode::Pull),
            clone_options: None,
            filters: FilterOptions {
                include_archived: false,
                include_forks: true,
                orgs: vec![],
                exclude_repos: vec![],
            },
            concurrency: Some(8),
            last_synced: Some("2026-02-23T10:00:00Z".to_string()),
        };

        let toml_str = ws.to_toml().unwrap();
        let parsed = WorkspaceConfig::from_toml(&toml_str).unwrap();

        // name is skip_serializing — it's derived from the folder, not the TOML
        assert!(parsed.name.is_empty());
        assert_eq!(parsed.base_path, ws.base_path);
        assert_eq!(parsed.username, ws.username);
        assert_eq!(parsed.orgs, ws.orgs);
        assert_eq!(parsed.exclude_repos, ws.exclude_repos);
        assert_eq!(parsed.structure, ws.structure);
        assert_eq!(parsed.sync_mode, ws.sync_mode);
        assert_eq!(parsed.concurrency, ws.concurrency);
        assert_eq!(parsed.last_synced, ws.last_synced);
        assert_eq!(parsed.provider.kind, ws.provider.kind);
        assert_eq!(parsed.provider.auth, ws.provider.auth);
        assert!(parsed.filters.include_forks);
    }

    #[test]
    fn test_expanded_base_path() {
        let ws = WorkspaceConfig::new("test", "~/github");
        let expanded = ws.expanded_base_path();
        assert!(!expanded.to_string_lossy().contains('~'));
    }

    #[test]
    fn test_summary() {
        let ws = WorkspaceConfig {
            orgs: vec!["org1".to_string(), "org2".to_string()],
            last_synced: None,
            ..WorkspaceConfig::new("github", "~/github")
        };
        let summary = ws.summary();
        assert!(summary.contains("github"));
        assert!(summary.contains("2 org(s)"));
        assert!(summary.contains("never synced"));
    }

    #[test]
    fn test_summary_all_orgs() {
        let ws = WorkspaceConfig::new("work", "~/work");
        let summary = ws.summary();
        assert!(summary.contains("all orgs"));
    }

    #[test]
    fn test_optional_fields_not_serialized_when_none() {
        let ws = WorkspaceConfig::new("minimal", "~/minimal");
        let toml_str = ws.to_toml().unwrap();
        // name is derived from folder, never written to TOML as its own key
        assert!(
            !toml_str.lines().any(|l| l.starts_with("name ")),
            "TOML should not contain a 'name' key"
        );
        assert!(!toml_str.contains("structure"));
        assert!(!toml_str.contains("sync_mode"));
        assert!(!toml_str.contains("concurrency"));
        assert!(!toml_str.contains("last_synced"));
    }
}
