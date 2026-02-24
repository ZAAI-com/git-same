//! Configuration file parser.
//!
//! Handles loading and parsing of gisa.config.toml files.

use super::provider_config::ProviderEntry;
use crate::errors::AppError;
use crate::operations::clone::DEFAULT_CONCURRENCY;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Clone-specific configuration options (from config file).
///
/// Note: This is distinct from `git::CloneOptions` which is used for
/// the actual git clone operation parameters.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConfigCloneOptions {
    /// Shallow clone depth (0 = full history)
    #[serde(default)]
    pub depth: u32,

    /// Specific branch to clone (empty = default branch)
    #[serde(default)]
    pub branch: String,

    /// Whether to clone submodules
    #[serde(default)]
    pub recurse_submodules: bool,
}

/// Repository filter options.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FilterOptions {
    /// Include archived repositories
    #[serde(default)]
    pub include_archived: bool,

    /// Include forked repositories
    #[serde(default)]
    pub include_forks: bool,

    /// Filter to specific organizations (empty = all)
    #[serde(default)]
    pub orgs: Vec<String>,

    /// Exclude specific repos by full name (e.g., "org/repo")
    #[serde(default)]
    pub exclude_repos: Vec<String>,
}

/// Sync mode for existing repositories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum SyncMode {
    /// Only fetch (safe, doesn't modify working tree)
    #[default]
    Fetch,
    /// Pull changes (modifies working tree)
    Pull,
}

impl std::str::FromStr for SyncMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "fetch" => Ok(SyncMode::Fetch),
            "pull" => Ok(SyncMode::Pull),
            _ => Err(format!("Invalid sync mode: '{}'. Use 'fetch' or 'pull'", s)),
        }
    }
}

/// Full application configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Directory structure pattern
    /// Placeholders: {provider}, {org}, {repo}
    #[serde(default = "default_structure")]
    pub structure: String,

    /// Number of parallel operations
    #[serde(default = "default_concurrency")]
    pub concurrency: usize,

    /// Sync behavior
    #[serde(default)]
    pub sync_mode: SyncMode,

    /// Default workspace name (used when --workspace is not specified and multiple exist)
    #[serde(default)]
    pub default_workspace: Option<String>,

    /// Clone options
    #[serde(default)]
    #[serde(rename = "clone")]
    pub clone: ConfigCloneOptions,

    /// Filter options
    #[serde(default)]
    pub filters: FilterOptions,

    /// Provider configurations
    #[serde(default = "default_providers")]
    pub providers: Vec<ProviderEntry>,
}

fn default_structure() -> String {
    "{org}/{repo}".to_string()
}

fn default_concurrency() -> usize {
    DEFAULT_CONCURRENCY
}

fn default_providers() -> Vec<ProviderEntry> {
    vec![ProviderEntry::github()]
}

impl Default for Config {
    fn default() -> Self {
        Self {
            structure: default_structure(),
            concurrency: default_concurrency(),
            sync_mode: SyncMode::default(),
            default_workspace: None,
            clone: ConfigCloneOptions::default(),
            filters: FilterOptions::default(),
            providers: default_providers(),
        }
    }
}

impl Config {
    /// Returns the default config file path (~/.config/git-same/config.toml).
    pub fn default_path() -> Result<PathBuf, AppError> {
        #[cfg(target_os = "macos")]
        let config_dir = {
            let home = std::env::var("HOME")
                .map_err(|_| AppError::config("HOME environment variable not set"))?;
            PathBuf::from(home).join(".config/git-same")
        };
        #[cfg(not(target_os = "macos"))]
        let config_dir = if let Some(dir) = directories::ProjectDirs::from("", "", "git-same") {
            dir.config_dir().to_path_buf()
        } else {
            let home = std::env::var("HOME")
                .map_err(|_| AppError::config("HOME environment variable not set"))?;
            PathBuf::from(home).join(".config/git-same")
        };

        Ok(config_dir.join("config.toml"))
    }

    /// Load configuration from the default path, or return defaults if file doesn't exist.
    pub fn load() -> Result<Self, AppError> {
        Self::load_from(&Self::default_path()?)
    }

    /// Load configuration from a specific file, or return defaults if file doesn't exist.
    pub fn load_from(path: &Path) -> Result<Self, AppError> {
        if path.exists() {
            let content = std::fs::read_to_string(path)
                .map_err(|e| AppError::config(format!("Failed to read config file: {}", e)))?;
            Self::parse(&content)
        } else {
            Ok(Config::default())
        }
    }

    /// Parse configuration from a TOML string.
    pub fn parse(content: &str) -> Result<Self, AppError> {
        let config: Config = toml::from_str(content)
            .map_err(|e| AppError::config(format!("Failed to parse config: {}", e)))?;
        config.validate()?;
        Ok(config)
    }

    /// Validate the configuration.
    pub fn validate(&self) -> Result<(), AppError> {
        const MAX_CONCURRENCY: usize = 32;

        // Validate concurrency
        if !(1..=MAX_CONCURRENCY).contains(&self.concurrency) {
            return Err(AppError::config(format!(
                "concurrency must be between 1 and {}",
                MAX_CONCURRENCY
            )));
        }

        // Validate providers
        if self.providers.is_empty() {
            return Err(AppError::config("At least one provider must be configured"));
        }

        for (i, provider) in self.providers.iter().enumerate() {
            provider
                .validate()
                .map_err(|e| AppError::config(format!("Provider {} error: {}", i + 1, e)))?;
        }

        Ok(())
    }

    /// Generate the default configuration file content.
    pub fn default_toml() -> String {
        format!(
            r#"# Git-Same Configuration
# See: https://github.com/zaai-com/git-same

# Directory structure pattern
# Placeholders: {{provider}}, {{org}}, {{repo}}
structure = "{{org}}/{{repo}}"

# Number of parallel clone/sync operations (1-32)
# Keeping this bounded helps avoid provider rate limits and local resource contention.
concurrency = {}

# Sync behavior: "fetch" (safe) or "pull" (updates working tree)
sync_mode = "fetch""#,
            DEFAULT_CONCURRENCY
        ) + r#"

[clone]
# Clone depth (0 = full history)
depth = 0

# Clone submodules
recurse_submodules = false

[filters]
# Include archived repositories
include_archived = false

# Include forked repositories
include_forks = false

# Filter to specific organizations (empty = all)
# orgs = ["my-org", "other-org"]

# Exclude specific repos
# exclude_repos = ["org/repo-to-skip"]

# Provider configuration (default: GitHub.com with gh CLI auth)
[[providers]]
kind = "github"
auth = "gh-cli"
prefer_ssh = true

# Example: GitHub Enterprise
# [[providers]]
# kind = "github-enterprise"
# name = "Work GitHub"
# api_url = "https://github.mycompany.com/api/v3"
# auth = "env"
# token_env = "WORK_GITHUB_TOKEN"
# base_path = "~/work/code"
"#
    }

    /// Save the default_workspace setting to the config file at the default path.
    pub fn save_default_workspace(workspace: Option<&str>) -> Result<(), AppError> {
        Self::save_default_workspace_to(&Self::default_path()?, workspace)
    }

    /// Save the default_workspace setting to a specific config file.
    ///
    /// Uses targeted text replacement to preserve comments and formatting.
    pub fn save_default_workspace_to(path: &Path, workspace: Option<&str>) -> Result<(), AppError> {
        let content = if path.exists() {
            std::fs::read_to_string(path)
                .map_err(|e| AppError::config(format!("Failed to read config: {}", e)))?
        } else {
            return Err(AppError::config(
                "Config file not found. Run 'gisa init' first.",
            ));
        };

        let new_line = match workspace {
            Some(name) => format!("default_workspace = \"{}\"", name),
            None => String::new(),
        };

        // Replace existing default_workspace line, or insert after sync_mode
        let new_content = if content.contains("default_workspace") {
            let mut lines: Vec<&str> = content.lines().collect();
            lines.retain(|line| {
                let trimmed = line.trim();
                !trimmed.starts_with("default_workspace")
                    && !trimmed.starts_with("# default_workspace")
            });
            let mut result = lines.join("\n");
            if !new_line.is_empty() {
                // Insert after sync_mode line
                if let Some(pos) = result.find("sync_mode") {
                    if let Some(nl) = result[pos..].find('\n') {
                        let insert_pos = pos + nl + 1;
                        result.insert_str(insert_pos, &format!("{}\n", new_line));
                    }
                }
            }
            // Ensure trailing newline
            if !result.ends_with('\n') {
                result.push('\n');
            }
            result
        } else if !new_line.is_empty() {
            // Insert after sync_mode line
            let mut result = content.clone();
            if let Some(pos) = result.find("sync_mode") {
                if let Some(nl) = result[pos..].find('\n') {
                    let insert_pos = pos + nl + 1;
                    result.insert_str(insert_pos, &format!("\n{}\n", new_line));
                }
            } else {
                // Fallback: insert near the top (after first blank line)
                if let Some(pos) = result.find("\n\n") {
                    result.insert_str(pos + 1, &format!("\n{}\n", new_line));
                } else {
                    result = format!("{}\n{}\n", new_line, result);
                }
            }
            result
        } else {
            // Nothing to do — clearing a field that doesn't exist
            content
        };

        std::fs::write(path, new_content)
            .map_err(|e| AppError::config(format!("Failed to write config: {}", e)))?;
        Ok(())
    }

    /// Returns enabled providers only.
    pub fn enabled_providers(&self) -> impl Iterator<Item = &ProviderEntry> {
        self.providers.iter().filter(|p| p.enabled)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.concurrency, 8);
        assert_eq!(config.sync_mode, SyncMode::Fetch);
        assert!(!config.filters.include_archived);
        assert!(!config.filters.include_forks);
        assert_eq!(config.providers.len(), 1);
    }

    #[test]
    fn test_load_minimal_config() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "concurrency = 2").unwrap();

        let config = Config::load_from(file.path()).unwrap();
        assert_eq!(config.concurrency, 2);
    }

    #[test]
    fn test_load_full_config() {
        let content = r#"
structure = "{provider}/{org}/{repo}"
concurrency = 8
sync_mode = "pull"

[clone]
depth = 1
recurse_submodules = true

[filters]
include_archived = true
include_forks = true
orgs = ["my-org"]
exclude_repos = ["my-org/skip-this"]

[[providers]]
kind = "github"
auth = "gh-cli"
"#;

        let config = Config::parse(content).unwrap();
        assert_eq!(config.structure, "{provider}/{org}/{repo}");
        assert_eq!(config.concurrency, 8);
        assert_eq!(config.sync_mode, SyncMode::Pull);
        assert_eq!(config.clone.depth, 1);
        assert!(config.clone.recurse_submodules);
        assert!(config.filters.include_archived);
        assert!(config.filters.include_forks);
        assert_eq!(config.filters.orgs, vec!["my-org"]);
        assert_eq!(config.filters.exclude_repos, vec!["my-org/skip-this"]);
    }

    #[test]
    fn test_load_multi_provider_config() {
        let content = r#"
[[providers]]
kind = "github"
auth = "gh-cli"

[[providers]]
kind = "github-enterprise"
name = "Work"
api_url = "https://github.work.com/api/v3"
auth = "env"
token_env = "WORK_TOKEN"
"#;

        let config = Config::parse(content).unwrap();
        assert_eq!(config.providers.len(), 2);
        assert_eq!(config.providers[0].kind, crate::types::ProviderKind::GitHub);
        assert_eq!(
            config.providers[1].kind,
            crate::types::ProviderKind::GitHubEnterprise
        );
        assert_eq!(config.providers[1].name, Some("Work".to_string()));
    }

    #[test]
    fn test_missing_file_returns_defaults() {
        let config = Config::load_from(Path::new("/nonexistent/config.toml")).unwrap();
        assert_eq!(config.concurrency, 8);
    }

    #[test]
    fn test_validation_rejects_zero_concurrency() {
        let config = Config {
            concurrency: 0,
            ..Config::default()
        };
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("concurrency"));
    }

    #[test]
    fn test_validation_rejects_high_concurrency() {
        let config = Config {
            concurrency: 100,
            ..Config::default()
        };
        let result = config.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_validation_rejects_empty_providers() {
        let config = Config {
            providers: vec![],
            ..Config::default()
        };
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("provider"));
    }

    #[test]
    fn test_sync_mode_from_str() {
        assert_eq!("fetch".parse::<SyncMode>().unwrap(), SyncMode::Fetch);
        assert_eq!("pull".parse::<SyncMode>().unwrap(), SyncMode::Pull);
        assert_eq!("FETCH".parse::<SyncMode>().unwrap(), SyncMode::Fetch);
        assert!("invalid".parse::<SyncMode>().is_err());
    }

    #[test]
    fn test_default_toml_is_valid() {
        let toml = Config::default_toml();
        let result = Config::parse(&toml);
        assert!(result.is_ok(), "Default TOML should be valid: {:?}", result);
    }

    #[test]
    fn test_enabled_providers_filter() {
        let config = Config {
            providers: vec![
                ProviderEntry {
                    enabled: true,
                    ..ProviderEntry::github()
                },
                ProviderEntry {
                    enabled: false,
                    ..ProviderEntry::github()
                },
                ProviderEntry {
                    enabled: true,
                    ..ProviderEntry::github()
                },
            ],
            ..Config::default()
        };

        let enabled: Vec<_> = config.enabled_providers().collect();
        assert_eq!(enabled.len(), 2);
    }

    #[test]
    fn test_default_config_has_no_default_workspace() {
        let config = Config::default();
        assert!(config.default_workspace.is_none());
    }

    #[test]
    fn test_parse_config_with_default_workspace() {
        let content = r#"
default_workspace = "my-ws"

[[providers]]
kind = "github"
auth = "gh-cli"
"#;
        let config = Config::parse(content).unwrap();
        assert_eq!(config.default_workspace, Some("my-ws".to_string()));
    }

    #[test]
    fn test_parse_config_without_default_workspace() {
        let content = r#"
[[providers]]
kind = "github"
auth = "gh-cli"
"#;
        let config = Config::parse(content).unwrap();
        assert!(config.default_workspace.is_none());
    }

    #[test]
    fn test_save_default_workspace_to_set() {
        let temp = tempfile::TempDir::new().unwrap();
        let path = temp.path().join("config.toml");
        std::fs::write(&path, Config::default_toml()).unwrap();

        Config::save_default_workspace_to(&path, Some("my-ws")).unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("default_workspace = \"my-ws\""));
        // Original content preserved
        assert!(content.contains("concurrency"));
        // Still valid TOML
        let config = Config::parse(&content).unwrap();
        assert_eq!(config.default_workspace, Some("my-ws".to_string()));
    }

    #[test]
    fn test_save_default_workspace_to_clear() {
        let temp = tempfile::TempDir::new().unwrap();
        let path = temp.path().join("config.toml");
        std::fs::write(&path, Config::default_toml()).unwrap();

        // Set then clear
        Config::save_default_workspace_to(&path, Some("my-ws")).unwrap();
        Config::save_default_workspace_to(&path, None).unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(!content.contains("default_workspace"));
        // Still valid TOML
        let config = Config::parse(&content).unwrap();
        assert!(config.default_workspace.is_none());
    }

    #[test]
    fn test_save_default_workspace_to_replace() {
        let temp = tempfile::TempDir::new().unwrap();
        let path = temp.path().join("config.toml");
        std::fs::write(&path, Config::default_toml()).unwrap();

        Config::save_default_workspace_to(&path, Some("ws1")).unwrap();
        Config::save_default_workspace_to(&path, Some("ws2")).unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("default_workspace = \"ws2\""));
        assert!(!content.contains("ws1"));
        let config = Config::parse(&content).unwrap();
        assert_eq!(config.default_workspace, Some("ws2".to_string()));
    }

    #[test]
    fn test_save_default_workspace_to_nonexistent_file() {
        let result =
            Config::save_default_workspace_to(Path::new("/nonexistent/config.toml"), Some("ws"));
        assert!(result.is_err());
    }
}
