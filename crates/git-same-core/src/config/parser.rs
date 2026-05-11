//! Configuration file parser.
//!
//! Handles loading and parsing of config.toml files.

use crate::errors::AppError;
use crate::operations::clone::{DEFAULT_CONCURRENCY, MAX_CONCURRENCY};
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

/// Monitor process configuration.
///
/// Controls behavior of the long-running monitor (`gisa monitor`) that
/// periodically rescans every workspace and ambient root, then writes
/// `status.json` for the Finder extension to consume.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorConfig {
    /// Seconds between full rescans. The CLI flag `--interval` overrides this.
    #[serde(default = "default_fullscan_interval_secs")]
    pub fullscan_interval_secs: u64,
}

impl Default for MonitorConfig {
    fn default() -> Self {
        Self {
            fullscan_interval_secs: default_fullscan_interval_secs(),
        }
    }
}

fn default_fullscan_interval_secs() -> u64 {
    30
}

/// Finder-badge discovery configuration.
///
/// Controls how the monitor finds ambient git repositories outside any
/// configured workspace. Ambient repos get a neutral gray badge until the
/// user opens their context menu, which triggers an on-demand upgrade.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinderConfig {
    /// Roots to walk when looking for ambient git repos. Defaults to `["~"]`
    /// (the user's home directory).
    #[serde(default = "default_finder_scan_roots")]
    pub scan_roots: Vec<String>,

    /// Maximum directory depth to descend during the ambient walk.
    #[serde(default = "default_finder_max_depth")]
    pub max_depth: usize,

    /// Directory names to skip during the ambient walk. Load-bearing for
    /// performance: without entries like `node_modules` and `target`, the
    /// scan times balloon on developer machines.
    #[serde(default = "default_finder_excludes")]
    pub exclude_dirs: Vec<String>,

    /// Feature flag: when true, the monitor adds `scan_roots` to the watched
    /// directory set so ambient repos can be badged. Defaults to false because
    /// the default `scan_roots = ["~"]` expands to the user's home directory,
    /// and macOS silently refuses to deliver `requestBadgeIdentifier` calls to
    /// FinderSync extensions whose `directoryURLs` contain the home folder —
    /// which would suppress badges for the configured workspaces too. Opt in
    /// only after narrowing `scan_roots` to specific subdirectories.
    #[serde(default = "default_false")]
    pub show_ambient: bool,
}

impl Default for FinderConfig {
    fn default() -> Self {
        Self {
            scan_roots: default_finder_scan_roots(),
            max_depth: default_finder_max_depth(),
            exclude_dirs: default_finder_excludes(),
            show_ambient: false,
        }
    }
}

fn default_finder_scan_roots() -> Vec<String> {
    vec!["~".to_string()]
}

fn default_finder_max_depth() -> usize {
    8
}

fn default_finder_excludes() -> Vec<String> {
    vec![
        "node_modules".into(),
        "target".into(),
        "build".into(),
        "dist".into(),
        "DerivedData".into(),
        "Pods".into(),
        "Library".into(),
        ".cache".into(),
        ".cargo".into(),
        ".rustup".into(),
        ".npm".into(),
        ".yarn".into(),
        ".venv".into(),
        ".Trash".into(),
        ".git-same".into(),
        ".zsh_sessions".into(),
    ]
}

fn default_false() -> bool {
    false
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

    /// Default workspace path (used when --workspace is not specified and multiple exist)
    #[serde(default)]
    pub default_workspace: Option<String>,

    /// Dashboard auto-refresh interval in seconds (5–3600, default 30)
    #[serde(default = "default_refresh_interval")]
    pub refresh_interval: u64,

    /// Clone options
    #[serde(default)]
    #[serde(rename = "clone")]
    pub clone: ConfigCloneOptions,

    /// Filter options
    #[serde(default)]
    pub filters: FilterOptions,

    /// Registry of known workspace root paths (tilde-collapsed).
    #[serde(default)]
    pub workspaces: Vec<String>,

    /// Finder badge monitor configuration (ambient repo discovery).
    #[serde(default)]
    pub finder: FinderConfig,

    /// Monitor process configuration (scan cadence, etc.).
    #[serde(default)]
    pub monitor: MonitorConfig,
}

fn default_structure() -> String {
    "{org}/{repo}".to_string()
}

fn default_concurrency() -> usize {
    DEFAULT_CONCURRENCY
}

fn default_refresh_interval() -> u64 {
    30
}

impl Default for Config {
    fn default() -> Self {
        Self {
            structure: default_structure(),
            concurrency: default_concurrency(),
            sync_mode: SyncMode::default(),
            default_workspace: None,
            refresh_interval: default_refresh_interval(),
            clone: ConfigCloneOptions::default(),
            filters: FilterOptions::default(),
            workspaces: Vec::new(),
            finder: FinderConfig::default(),
            monitor: MonitorConfig::default(),
        }
    }
}

impl Config {
    /// Returns the default config file path (~/.config/git-same/config.toml).
    ///
    /// When `GIT_SAME_CONFIG_DIR` is set to an absolute path, that directory is used instead.
    /// This allows tests to override config location on Windows (where dirs-sys ignores APPDATA).
    pub fn default_path() -> Result<PathBuf, AppError> {
        if let Ok(override_dir) = std::env::var("GIT_SAME_CONFIG_DIR") {
            let dir = PathBuf::from(&override_dir);
            if dir.is_absolute() {
                return Ok(dir.join("config.toml"));
            }
        }

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
                .or_else(|_| std::env::var("USERPROFILE"))
                .map_err(|_| {
                    AppError::config("Neither HOME nor USERPROFILE environment variable is set")
                })?;
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
        // Validate concurrency
        if !(1..=MAX_CONCURRENCY).contains(&self.concurrency) {
            return Err(AppError::config(format!(
                "concurrency must be between 1 and {}",
                MAX_CONCURRENCY
            )));
        }

        // Validate refresh_interval
        if !(5..=3600).contains(&self.refresh_interval) {
            return Err(AppError::config(
                "refresh_interval must be between 5 and 3600 seconds",
            ));
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

# Number of parallel clone/sync operations (1-{})
# Keeping this bounded helps avoid provider rate limits and local resource contention.
concurrency = {}

# Sync behavior: "fetch" (safe) or "pull" (updates working tree)
sync_mode = "fetch""#,
            MAX_CONCURRENCY, DEFAULT_CONCURRENCY
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

[finder]
# Show a neutral gray R-badge on every git repository found under
# `scan_roots`, even outside a configured workspace. Right-clicking a gray
# repo upgrades it to the normal color (green/blue/orange/red).
#
# Disabled by default because the default `scan_roots = ["~"]` would put the
# user's home folder into the FinderSync extension's watched URL set, and
# macOS silently refuses to deliver badge requests to extensions that watch
# the home directory — that suppresses badges everywhere, including the
# configured workspaces. Narrow `scan_roots` to specific subdirectories
# before enabling this.
show_ambient = false

# Roots to walk for ambient repos. "~" expands to your home directory.
# Only used when `show_ambient = true`. Don't leave this as `["~"]` if you
# enable show_ambient — see the note above.
scan_roots = ["~"]

# Maximum directory depth for the ambient walk.
max_depth = 8

# Directory names skipped during the ambient walk. Critical for performance.
exclude_dirs = [
    "node_modules", "target", "build", "dist", "DerivedData", "Pods",
    "Library", ".cache", ".cargo", ".rustup", ".npm", ".yarn", ".venv",
    ".Trash", ".git-same", ".zsh_sessions",
]

[monitor]
# Seconds between full rescans by the background monitor. The CLI flag
# `gisa monitor --interval N` overrides this when set explicitly.
fullscan_interval_secs = 30
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
            Some(name) => {
                let escaped = toml::Value::String(name.to_string()).to_string();
                format!("default_workspace = {}", escaped)
            }
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
                } else {
                    // Fallback: insert near the top (after first blank line)
                    if let Some(pos) = result.find("\n\n") {
                        result.insert_str(pos + 1, &format!("\n{}\n", new_line));
                    } else {
                        result = format!("{}\n{}\n", new_line, result);
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

    /// Add a workspace path to the global registry.
    pub fn add_to_registry(path: &str) -> Result<(), AppError> {
        Self::add_to_registry_at(&Self::default_path()?, path)
    }

    /// Add a workspace path to the registry in a specific config file.
    pub fn add_to_registry_at(config_path: &Path, path: &str) -> Result<(), AppError> {
        if !config_path.exists() {
            return Err(AppError::config(
                "Config file not found. Run 'gisa init' first.",
            ));
        }
        Self::modify_registry_at(config_path, Some(path), None)
    }

    /// Remove a workspace path from the global registry.
    pub fn remove_from_registry(path: &str) -> Result<(), AppError> {
        Self::remove_from_registry_at(&Self::default_path()?, path)
    }

    /// Remove a workspace path from the registry in a specific config file.
    pub fn remove_from_registry_at(config_path: &Path, path: &str) -> Result<(), AppError> {
        if !config_path.exists() {
            return Ok(());
        }
        Self::modify_registry_at(config_path, None, Some(path))
    }

    /// Add or remove a path from the workspaces registry in the config file.
    fn modify_registry_at(
        config_path: &Path,
        add: Option<&str>,
        remove: Option<&str>,
    ) -> Result<(), AppError> {
        let content = std::fs::read_to_string(config_path)
            .map_err(|e| AppError::config(format!("Failed to read config: {}", e)))?;

        let mut doc: toml::Value = toml::from_str(&content)
            .map_err(|e| AppError::config(format!("Failed to parse config: {}", e)))?;

        let table = doc
            .as_table_mut()
            .ok_or_else(|| AppError::config("Invalid config: expected root table"))?;

        if let Some(existing) = table.get("workspaces") {
            if !existing.is_array() {
                return Err(AppError::config(
                    "Invalid config: 'workspaces' must be an array",
                ));
            }
        }

        let workspaces = table
            .entry("workspaces")
            .or_insert_with(|| toml::Value::Array(Vec::new()));
        let arr = workspaces
            .as_array_mut()
            .ok_or_else(|| AppError::config("Invalid config: 'workspaces' must be an array"))?;

        if let Some(path_to_add) = add {
            let val = toml::Value::String(path_to_add.to_string());
            if !arr.contains(&val) {
                arr.push(val);
            }
        }
        if let Some(path_to_remove) = remove {
            arr.retain(|v| v.as_str().map(|s| s != path_to_remove).unwrap_or(true));
        }

        let new_content = toml::to_string_pretty(&doc)
            .map_err(|e| AppError::config(format!("Failed to serialize config: {}", e)))?;

        std::fs::write(config_path, new_content)
            .map_err(|e| AppError::config(format!("Failed to write config: {}", e)))?;

        Ok(())
    }
}

#[cfg(test)]
#[path = "parser_tests.rs"]
mod tests;
