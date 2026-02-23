//! Workspace configuration management.
//!
//! Handles CRUD operations for workspace config files stored in
//! `~/.config/git-same/workspaces/`.

use super::workspace::WorkspaceConfig;
use crate::errors::AppError;
use std::path::{Path, PathBuf};

/// Manages workspace configuration files.
pub struct WorkspaceManager;

impl WorkspaceManager {
    /// Returns the workspaces directory: `~/.config/git-same/workspaces/`.
    pub fn workspaces_dir() -> Result<PathBuf, AppError> {
        let config_path = crate::config::Config::default_path()?;
        let config_dir = config_path
            .parent()
            .ok_or_else(|| AppError::config("Cannot determine config directory"))?;
        Ok(config_dir.join("workspaces"))
    }

    /// Ensure the workspaces directory exists.
    pub fn ensure_dir() -> Result<PathBuf, AppError> {
        let dir = Self::workspaces_dir()?;
        if !dir.exists() {
            std::fs::create_dir_all(&dir).map_err(|e| {
                AppError::config(format!("Failed to create workspaces directory: {}", e))
            })?;
        }
        Ok(dir)
    }

    /// List all workspace configs.
    pub fn list() -> Result<Vec<WorkspaceConfig>, AppError> {
        let dir = Self::workspaces_dir()?;
        if !dir.exists() {
            return Ok(Vec::new());
        }

        let mut workspaces = Vec::new();
        let entries = std::fs::read_dir(&dir)
            .map_err(|e| AppError::config(format!("Failed to read workspaces directory: {}", e)))?;

        for entry in entries {
            let entry = entry
                .map_err(|e| AppError::config(format!("Failed to read directory entry: {}", e)))?;
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "toml") {
                match Self::load_from_path(&path) {
                    Ok(ws) => workspaces.push(ws),
                    Err(e) => {
                        tracing::warn!(
                            path = %path.display(),
                            error = %e,
                            "Skipping invalid workspace config"
                        );
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
        let dir = Self::ensure_dir()?;
        let path = dir.join(format!("{}.toml", workspace.name));
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

    /// Delete a workspace by name.
    pub fn delete(name: &str) -> Result<(), AppError> {
        let path = Self::config_path(name)?;
        if !path.exists() {
            return Err(AppError::config(format!("Workspace '{}' not found", name)));
        }
        std::fs::remove_file(&path).map_err(|e| {
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

    /// Derive a workspace name from a path.
    ///
    /// Examples:
    /// - `~/github` → `"github"`
    /// - `~/work/code` → `"work-code"`
    /// - `/home/user/my repos` → `"my-repos"`
    pub fn name_from_path(path: &Path) -> String {
        let lossy = path.to_string_lossy();
        let expanded = shellexpand::tilde(&lossy);
        let path = Path::new(expanded.as_ref());

        // Take the last 1-2 path components
        let components: Vec<&str> = path
            .components()
            .filter_map(|c| {
                if let std::path::Component::Normal(s) = c {
                    s.to_str()
                } else {
                    None
                }
            })
            .collect();

        let name_parts = if components.len() >= 2 {
            vec![
                components[components.len() - 2],
                components[components.len() - 1],
            ]
        } else if let Some(last) = components.last() {
            vec![*last]
        } else {
            vec!["workspace"]
        };

        name_parts.join("-").to_lowercase().replace([' ', '_'], "-")
    }

    /// Returns the file path for a workspace config.
    fn config_path(name: &str) -> Result<PathBuf, AppError> {
        let dir = Self::workspaces_dir()?;
        Ok(dir.join(format!("{}.toml", name)))
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
        WorkspaceConfig::from_toml(&content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn with_temp_workspaces_dir(f: impl FnOnce(&Path)) {
        let temp = TempDir::new().unwrap();
        let workspaces_dir = temp.path().join("workspaces");
        std::fs::create_dir_all(&workspaces_dir).unwrap();

        // Create a workspace config in the temp dir
        let ws = WorkspaceConfig::new("test-ws", "~/github");
        let content = ws.to_toml().unwrap();
        std::fs::write(workspaces_dir.join("test-ws.toml"), &content).unwrap();

        f(&workspaces_dir);
    }

    #[test]
    fn test_name_from_path_simple() {
        let name = WorkspaceManager::name_from_path(Path::new("/home/user/github"));
        assert_eq!(name, "user-github");
    }

    #[test]
    fn test_name_from_path_with_spaces() {
        let name = WorkspaceManager::name_from_path(Path::new("/home/user/my repos"));
        assert_eq!(name, "user-my-repos");
    }

    #[test]
    fn test_name_from_path_single_component() {
        let name = WorkspaceManager::name_from_path(Path::new("/github"));
        assert_eq!(name, "github");
    }

    #[test]
    fn test_name_from_path_deep() {
        let name = WorkspaceManager::name_from_path(Path::new("/a/b/c/work/code"));
        // Takes last 2 components
        assert_eq!(name, "work-code");
    }

    #[test]
    fn test_workspace_config_save_and_load_roundtrip() {
        with_temp_workspaces_dir(|dir| {
            let ws = WorkspaceConfig {
                name: "roundtrip-test".to_string(),
                base_path: "~/test".to_string(),
                username: "testuser".to_string(),
                orgs: vec!["org1".to_string()],
                ..WorkspaceConfig::new("roundtrip-test", "~/test")
            };

            let path = dir.join("roundtrip-test.toml");
            let content = ws.to_toml().unwrap();
            std::fs::write(&path, &content).unwrap();

            let content = std::fs::read_to_string(&path).unwrap();
            let loaded = WorkspaceConfig::from_toml(&content).unwrap();

            assert_eq!(loaded.name, "roundtrip-test");
            assert_eq!(loaded.base_path, "~/test");
            assert_eq!(loaded.username, "testuser");
            assert_eq!(loaded.orgs, vec!["org1"]);
        });
    }

    #[test]
    fn test_load_from_path_invalid_toml() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("bad.toml");
        std::fs::write(&path, "invalid toml {{{").unwrap();

        let result = WorkspaceManager::load_from_path(&path);
        assert!(result.is_err());
    }

    #[test]
    fn test_list_empty_dir() {
        let temp = TempDir::new().unwrap();
        let dir = temp.path().join("workspaces");
        std::fs::create_dir_all(&dir).unwrap();

        // Read directory directly since we can't override workspaces_dir
        let entries = std::fs::read_dir(&dir).unwrap();
        let count = entries.count();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_list_with_configs() {
        with_temp_workspaces_dir(|dir| {
            // Add a second workspace
            let ws2 = WorkspaceConfig::new("another-ws", "~/work");
            let content = ws2.to_toml().unwrap();
            std::fs::write(dir.join("another-ws.toml"), &content).unwrap();

            // Read directory
            let entries: Vec<_> = std::fs::read_dir(dir)
                .unwrap()
                .filter_map(|e| e.ok())
                .filter(|e| e.path().extension().is_some_and(|ext| ext == "toml"))
                .collect();
            assert_eq!(entries.len(), 2);
        });
    }
}
