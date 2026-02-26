//! Workspace persistence (storage concern only).

use super::workspace::WorkspaceConfig;
use crate::errors::AppError;
use std::path::{Path, PathBuf};

/// Filesystem-backed workspace store.
pub struct WorkspaceStore;

impl WorkspaceStore {
    /// Returns the config directory: `~/.config/git-same/`.
    pub fn config_dir() -> Result<PathBuf, AppError> {
        let config_path = crate::config::Config::default_path()?;
        config_path
            .parent()
            .map(|p| p.to_path_buf())
            .ok_or_else(|| AppError::config("Cannot determine config directory"))
    }

    /// List all workspace configs.
    pub fn list() -> Result<Vec<WorkspaceConfig>, AppError> {
        let dir = Self::config_dir()?;
        if !dir.exists() {
            return Ok(Vec::new());
        }

        let mut workspaces = Vec::new();
        let entries = std::fs::read_dir(&dir)
            .map_err(|e| AppError::config(format!("Failed to read config directory: {}", e)))?;

        for entry in entries {
            let entry = entry
                .map_err(|e| AppError::config(format!("Failed to read directory entry: {}", e)))?;
            let path = entry.path();
            if path.is_dir() {
                let config_file = path.join("workspace-config.toml");
                if config_file.exists() {
                    match Self::load_from_path(&config_file) {
                        Ok(ws) => workspaces.push(ws),
                        Err(e) => {
                            tracing::warn!(
                                path = %config_file.display(),
                                error = %e,
                                "Skipping invalid workspace config"
                            );
                        }
                    }
                }
            }
        }

        workspaces.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(workspaces)
    }

    /// Load a specific workspace by name.
    pub fn load(name: &str) -> Result<WorkspaceConfig, AppError> {
        let path = Self::config_path(name)?;
        if !path.exists() {
            return Err(AppError::config(format!(
                "Workspace '{}' not found at {}",
                name,
                path.display()
            )));
        }
        Self::load_from_path(&path)
    }

    /// Save a workspace config (create or update).
    pub fn save(workspace: &WorkspaceConfig) -> Result<(), AppError> {
        let path = Self::config_path(&workspace.name)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                AppError::config(format!("Failed to create workspace directory: {}", e))
            })?;
        }
        let content = workspace.to_toml()?;
        std::fs::write(&path, content).map_err(|e| {
            AppError::config(format!(
                "Failed to write workspace config at {}: {}",
                path.display(),
                e
            ))
        })?;
        Ok(())
    }

    /// Delete a workspace by name (removes the entire workspace directory).
    pub fn delete(name: &str) -> Result<(), AppError> {
        let dir = Self::workspace_dir(name)?;
        if !dir.exists() {
            return Err(AppError::config(format!("Workspace '{}' not found", name)));
        }
        std::fs::remove_dir_all(&dir).map_err(|e| {
            AppError::config(format!("Failed to delete workspace '{}': {}", name, e))
        })?;
        Ok(())
    }

    /// Find a workspace whose base_path matches the given directory.
    pub fn find_by_path(path: &Path) -> Result<Option<WorkspaceConfig>, AppError> {
        let workspaces = Self::list()?;
        let canonical = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());

        for ws in workspaces {
            let ws_path = ws.expanded_base_path();
            let ws_canonical = std::fs::canonicalize(&ws_path).unwrap_or_else(|_| ws_path.clone());
            if ws_canonical == canonical {
                return Ok(Some(ws));
            }
        }
        Ok(None)
    }

    /// Load a workspace by its base_path string.
    pub fn load_by_path(path_str: &str) -> Result<WorkspaceConfig, AppError> {
        let workspaces = Self::list()?;

        // Exact string match on base_path
        for ws in &workspaces {
            if ws.base_path == path_str {
                return Ok(ws.clone());
            }
        }

        // Canonical path comparison
        let expanded = shellexpand::tilde(path_str);
        let target = Path::new(expanded.as_ref());
        let target_canonical =
            std::fs::canonicalize(target).unwrap_or_else(|_| target.to_path_buf());

        for ws in workspaces {
            let ws_expanded = ws.expanded_base_path();
            let ws_canonical = std::fs::canonicalize(&ws_expanded).unwrap_or(ws_expanded);
            if ws_canonical == target_canonical {
                return Ok(ws);
            }
        }

        Err(AppError::config(format!(
            "No workspace configured for path '{}'",
            path_str
        )))
    }

    /// Returns the directory path for a workspace: `~/.config/git-same/<name>/`.
    pub fn workspace_dir(name: &str) -> Result<PathBuf, AppError> {
        Ok(Self::config_dir()?.join(name))
    }

    /// Returns the cache file path for a workspace: `~/.config/git-same/<name>/workspace-cache.json`.
    pub fn cache_path(name: &str) -> Result<PathBuf, AppError> {
        Ok(Self::workspace_dir(name)?.join("workspace-cache.json"))
    }

    /// Returns the file path for a workspace config.
    fn config_path(name: &str) -> Result<PathBuf, AppError> {
        Ok(Self::workspace_dir(name)?.join("workspace-config.toml"))
    }

    /// Load a workspace config from a specific file path.
    fn load_from_path(path: &Path) -> Result<WorkspaceConfig, AppError> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            AppError::config(format!(
                "Failed to read workspace config at {}: {}",
                path.display(),
                e
            ))
        })?;
        let mut ws = WorkspaceConfig::from_toml(&content)?;

        // Derive name from the parent folder
        if let Some(parent) = path.parent() {
            if let Some(folder_name) = parent.file_name().and_then(|n| n.to_str()) {
                ws.name = folder_name.to_string();
            }
        }

        Ok(ws)
    }
}
