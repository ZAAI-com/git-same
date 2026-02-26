//! Workspace persistence — stores workspace config inside the sync folder.
//!
//! Each workspace has a `.git-same/` directory inside its root that contains:
//! - `config.toml`       — workspace configuration
//! - `cache.json`        — discovery cache
//! - `sync-history.json` — sync history

use super::parser::Config;
use super::workspace::{tilde_collapse_path, WorkspaceConfig};
use crate::errors::AppError;
use std::path::{Path, PathBuf};

/// Name of the hidden workspace metadata directory.
pub const DOT_DIR: &str = ".git-same";
/// Config file name inside the `.git-same/` directory.
pub const CONFIG_FILE: &str = "config.toml";
/// Cache file name inside the `.git-same/` directory.
pub const CACHE_FILE: &str = "cache.json";
/// Sync history file name inside the `.git-same/` directory.
pub const SYNC_HISTORY_FILE: &str = "sync-history.json";

/// Filesystem-backed workspace store.
pub struct WorkspaceStore;

impl WorkspaceStore {
    /// Returns the `.git-same/` directory for a workspace root.
    pub fn dot_dir(root: &Path) -> PathBuf {
        root.join(DOT_DIR)
    }

    /// Returns the config file path for a workspace root.
    pub fn config_path(root: &Path) -> PathBuf {
        Self::dot_dir(root).join(CONFIG_FILE)
    }

    /// Returns the cache file path for a workspace root.
    pub fn cache_path(root: &Path) -> PathBuf {
        Self::dot_dir(root).join(CACHE_FILE)
    }

    /// Returns the sync history file path for a workspace root.
    pub fn sync_history_path(root: &Path) -> PathBuf {
        Self::dot_dir(root).join(SYNC_HISTORY_FILE)
    }

    /// Load a workspace config from the given root directory.
    ///
    /// Reads `<root>/.git-same/config.toml` and sets `root_path` from the directory.
    pub fn load(root: &Path) -> Result<WorkspaceConfig, AppError> {
        let expanded = expand_path(root);
        let config_path = Self::config_path(&expanded);
        if !config_path.exists() {
            return Err(AppError::config(format!(
                "No workspace config found at '{}'",
                config_path.display()
            )));
        }
        Self::load_from_path(&config_path)
    }

    /// Save a workspace config to `<root>/.git-same/config.toml`.
    ///
    /// Creates the `.git-same/` directory if necessary and registers the workspace
    /// in the global config registry.
    pub fn save(workspace: &WorkspaceConfig) -> Result<(), AppError> {
        let dot_dir = Self::dot_dir(&workspace.root_path);
        std::fs::create_dir_all(&dot_dir).map_err(|e| {
            AppError::config(format!(
                "Failed to create workspace directory '{}': {}",
                dot_dir.display(),
                e
            ))
        })?;

        let config_path = dot_dir.join(CONFIG_FILE);
        let content = workspace.to_toml()?;
        std::fs::write(&config_path, content).map_err(|e| {
            AppError::config(format!(
                "Failed to write workspace config at '{}': {}",
                config_path.display(),
                e
            ))
        })?;

        // Register in global config
        let tilde_path = tilde_collapse_path(&workspace.root_path);
        let _ = Config::add_to_registry(&tilde_path);

        Ok(())
    }

    /// List all registered workspace configs.
    ///
    /// Reads the global `workspaces` registry and loads each entry.
    /// Stale entries (where the config file no longer exists) are silently skipped.
    pub fn list() -> Result<Vec<WorkspaceConfig>, AppError> {
        let global = Config::load()?;
        let mut workspaces = Vec::new();

        for path_str in &global.workspaces {
            let expanded = shellexpand::tilde(path_str);
            let root = Path::new(expanded.as_ref());
            let config_path = Self::config_path(root);
            if !config_path.exists() {
                tracing::debug!(
                    path = %path_str,
                    "Skipping stale workspace registry entry"
                );
                continue;
            }
            match Self::load_from_path(&config_path) {
                Ok(ws) => workspaces.push(ws),
                Err(e) => {
                    tracing::warn!(
                        path = %config_path.display(),
                        error = %e,
                        "Skipping invalid workspace config"
                    );
                }
            }
        }

        Ok(workspaces)
    }

    /// Delete a workspace by removing its `.git-same/` directory.
    ///
    /// Also removes the workspace from the global registry.
    pub fn delete(root: &Path) -> Result<(), AppError> {
        let dot_dir = Self::dot_dir(root);
        if !dot_dir.exists() {
            return Err(AppError::config(format!(
                "No workspace config found at '{}'",
                dot_dir.display()
            )));
        }
        std::fs::remove_dir_all(&dot_dir).map_err(|e| {
            AppError::config(format!(
                "Failed to remove workspace at '{}': {}",
                dot_dir.display(),
                e
            ))
        })?;

        // Unregister from global config
        let tilde_path = tilde_collapse_path(root);
        let _ = Config::remove_from_registry(&tilde_path);

        Ok(())
    }

    /// Load a workspace config from a specific `.git-same/config.toml` path.
    ///
    /// Sets `root_path` from the parent of the `.git-same/` directory.
    pub fn load_from_path(config_path: &Path) -> Result<WorkspaceConfig, AppError> {
        let content = std::fs::read_to_string(config_path).map_err(|e| {
            AppError::config(format!(
                "Failed to read workspace config at '{}': {}",
                config_path.display(),
                e
            ))
        })?;
        let mut ws = WorkspaceConfig::from_toml(&content)?;

        // Derive root_path: parent of `.git-same/` directory
        // config_path = <root>/.git-same/config.toml
        // parent = <root>/.git-same/
        // parent.parent = <root>/
        if let Some(dot_dir) = config_path.parent() {
            if let Some(root) = dot_dir.parent() {
                ws.root_path = std::fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
            }
        }

        Ok(ws)
    }
}

/// Expand a path: resolve `~` and make absolute.
fn expand_path(path: &Path) -> PathBuf {
    let s = path.to_string_lossy();
    let expanded = shellexpand::tilde(&s);
    let p = Path::new(expanded.as_ref());
    std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf())
}

#[cfg(test)]
#[path = "workspace_store_tests.rs"]
mod tests;
