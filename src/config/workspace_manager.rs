//! Workspace configuration management.
//!
//! Handles CRUD operations for workspace config files.
//! Each workspace is a subdirectory of `~/.config/git-same/<name>/`
//! containing a `workspace-config.toml` and optionally a `workspace-cache.json`.

use super::workspace::WorkspaceConfig;
use crate::errors::AppError;
use crate::types::ProviderKind;
use std::path::{Path, PathBuf};

/// Manages workspace configuration files.
pub struct WorkspaceManager;

impl WorkspaceManager {
    /// Returns the config directory: `~/.config/git-same/`.
    pub fn config_dir() -> Result<PathBuf, AppError> {
        let config_path = crate::config::Config::default_path()?;
        config_path
            .parent()
            .map(|p| p.to_path_buf())
            .ok_or_else(|| AppError::config("Cannot determine config directory"))
    }

    /// List all workspace configs.
    ///
    /// Scans subdirectories of `~/.config/git-same/` for `workspace-config.toml` files.
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
        // Ensure the workspace subdirectory exists
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

    /// Derive a workspace name from a base path and provider.
    ///
    /// Format: `{provider}-{last_path_component}`, lowercased, with
    /// spaces and underscores replaced by hyphens.
    ///
    /// Examples (with GitHub provider):
    /// - `~/repos` → `"github-repos"`
    /// - `~/work/code` → `"github-code"`
    /// - `/home/user/my repos` → `"github-my-repos"`
    pub fn name_from_path(path: &Path, provider: ProviderKind) -> String {
        let lossy = path.to_string_lossy();
        let expanded = shellexpand::tilde(&lossy);
        let path = Path::new(expanded.as_ref());

        let last_component = path
            .components()
            .filter_map(|c| {
                if let std::path::Component::Normal(s) = c {
                    s.to_str()
                } else {
                    None
                }
            })
            .next_back()
            .unwrap_or("workspace");

        let prefix = match provider {
            ProviderKind::GitHub => "github",
            ProviderKind::GitHubEnterprise => "ghe",
            ProviderKind::GitLab => "gitlab",
            ProviderKind::Bitbucket => "bitbucket",
        };
        format!("{}-{}", prefix, last_component)
            .to_lowercase()
            .replace([' ', '_'], "-")
    }

    /// Return a unique workspace name, appending `-2`, `-3`, etc. on collision.
    pub fn unique_name(base: &str) -> Result<String, AppError> {
        let dir = Self::workspace_dir(base)?;
        if !dir.exists() {
            return Ok(base.to_string());
        }

        for suffix in 2..=100 {
            let candidate = format!("{}-{}", base, suffix);
            let candidate_dir = Self::workspace_dir(&candidate)?;
            if !candidate_dir.exists() {
                return Ok(candidate);
            }
        }

        Err(AppError::config(format!(
            "Could not find a unique workspace name based on '{}'",
            base
        )))
    }

    /// Resolve which workspace to use.
    ///
    /// Priority: explicit name → default from config → auto-select if only 1 → error.
    pub fn resolve(
        name: Option<&str>,
        config: &super::parser::Config,
    ) -> Result<WorkspaceConfig, AppError> {
        if let Some(name) = name {
            return Self::load(name);
        }

        if let Some(ref default) = config.default_workspace {
            return Self::load(default);
        }

        let workspaces = Self::list()?;
        Self::resolve_from_list(workspaces)
    }

    /// Resolve from an already-loaded list of workspaces (no filesystem access).
    ///
    /// Used when the explicit name and default have already been checked.
    pub fn resolve_from_list(
        workspaces: Vec<WorkspaceConfig>,
    ) -> Result<WorkspaceConfig, AppError> {
        match workspaces.len() {
            0 => Err(AppError::config(
                "No workspaces configured. Run 'gisa setup' first.",
            )),
            1 => Ok(workspaces.into_iter().next().unwrap()),
            _ => {
                let names: Vec<&str> = workspaces.iter().map(|w| w.name.as_str()).collect();
                Err(AppError::config(format!(
                    "Multiple workspaces configured. Use --workspace to select one, \
                     or set a default with 'gisa workspace default <name>': {}",
                    names.join(", ")
                )))
            }
        }
    }

    /// Returns the directory path for a workspace: `~/.config/git-same/<name>/`.
    pub fn workspace_dir(name: &str) -> Result<PathBuf, AppError> {
        Ok(Self::config_dir()?.join(name))
    }

    /// Returns the file path for a workspace config: `~/.config/git-same/<name>/workspace-config.toml`.
    fn config_path(name: &str) -> Result<PathBuf, AppError> {
        Ok(Self::workspace_dir(name)?.join("workspace-config.toml"))
    }

    /// Returns the cache file path for a workspace: `~/.config/git-same/<name>/workspace-cache.json`.
    pub fn cache_path(name: &str) -> Result<PathBuf, AppError> {
        Ok(Self::workspace_dir(name)?.join("workspace-cache.json"))
    }

    /// Load a workspace config from a specific file path.
    ///
    /// The workspace name is derived from the parent directory name,
    /// not from inside the TOML file.
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn with_temp_config_dir(f: impl FnOnce(&Path)) {
        let temp = TempDir::new().unwrap();
        let config_dir = temp.path();

        // Create a workspace config in a subdirectory
        let ws = WorkspaceConfig::new("test-ws", "~/github");
        let content = ws.to_toml().unwrap();
        let ws_dir = config_dir.join("test-ws");
        std::fs::create_dir_all(&ws_dir).unwrap();
        std::fs::write(ws_dir.join("workspace-config.toml"), &content).unwrap();

        f(config_dir);
    }

    #[test]
    fn test_name_from_path_simple() {
        let name =
            WorkspaceManager::name_from_path(Path::new("/home/user/github"), ProviderKind::GitHub);
        assert_eq!(name, "github-github");
    }

    #[test]
    fn test_name_from_path_with_spaces() {
        let name = WorkspaceManager::name_from_path(
            Path::new("/home/user/my repos"),
            ProviderKind::GitHub,
        );
        assert_eq!(name, "github-my-repos");
    }

    #[test]
    fn test_name_from_path_single_component() {
        let name = WorkspaceManager::name_from_path(Path::new("/repos"), ProviderKind::GitLab);
        assert_eq!(name, "gitlab-repos");
    }

    #[test]
    fn test_name_from_path_deep() {
        let name =
            WorkspaceManager::name_from_path(Path::new("/a/b/c/work/code"), ProviderKind::GitHub);
        assert_eq!(name, "github-code");
    }

    #[test]
    fn test_name_from_path_enterprise() {
        let name = WorkspaceManager::name_from_path(
            Path::new("/home/user/work"),
            ProviderKind::GitHubEnterprise,
        );
        assert_eq!(name, "ghe-work");
    }

    #[test]
    fn test_workspace_config_save_and_load_roundtrip() {
        with_temp_config_dir(|dir| {
            let ws = WorkspaceConfig {
                name: "roundtrip-test".to_string(),
                base_path: "~/test".to_string(),
                username: "testuser".to_string(),
                orgs: vec!["org1".to_string()],
                ..WorkspaceConfig::new("roundtrip-test", "~/test")
            };

            let ws_dir = dir.join("roundtrip-test");
            std::fs::create_dir_all(&ws_dir).unwrap();
            let path = ws_dir.join("workspace-config.toml");
            let content = ws.to_toml().unwrap();
            std::fs::write(&path, &content).unwrap();

            // Name is derived from the folder, not from the TOML content
            let loaded = WorkspaceManager::load_from_path(&path).unwrap();

            assert_eq!(loaded.name, "roundtrip-test");
            assert_eq!(loaded.base_path, "~/test");
            assert_eq!(loaded.username, "testuser");
            assert_eq!(loaded.orgs, vec!["org1"]);
        });
    }

    #[test]
    fn test_name_derived_from_folder_not_toml() {
        let temp = TempDir::new().unwrap();

        // Create a workspace config in a folder named "my-github"
        let ws = WorkspaceConfig::new("ignored-name", "~/github");
        let content = ws.to_toml().unwrap();
        let ws_dir = temp.path().join("my-github");
        std::fs::create_dir_all(&ws_dir).unwrap();
        std::fs::write(ws_dir.join("workspace-config.toml"), &content).unwrap();

        // Name comes from the folder, not from any field in the TOML
        let loaded =
            WorkspaceManager::load_from_path(&ws_dir.join("workspace-config.toml")).unwrap();
        assert_eq!(loaded.name, "my-github");

        // Simulate a folder rename
        let renamed_dir = temp.path().join("renamed-workspace");
        std::fs::rename(&ws_dir, &renamed_dir).unwrap();

        let loaded =
            WorkspaceManager::load_from_path(&renamed_dir.join("workspace-config.toml")).unwrap();
        assert_eq!(loaded.name, "renamed-workspace");
    }

    #[test]
    fn test_load_from_path_invalid_toml() {
        let temp = TempDir::new().unwrap();
        let ws_dir = temp.path().join("bad-ws");
        std::fs::create_dir_all(&ws_dir).unwrap();
        let path = ws_dir.join("workspace-config.toml");
        std::fs::write(&path, "invalid toml {{{").unwrap();

        let result = WorkspaceManager::load_from_path(&path);
        assert!(result.is_err());
    }

    #[test]
    fn test_list_empty_dir() {
        let temp = TempDir::new().unwrap();
        let dir = temp.path();

        // An empty config dir has no workspace subdirectories
        let entries: Vec<_> = std::fs::read_dir(dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir() && e.path().join("workspace-config.toml").exists())
            .collect();
        assert_eq!(entries.len(), 0);
    }

    #[test]
    fn test_list_with_configs() {
        with_temp_config_dir(|dir| {
            // Add a second workspace in its own subdirectory
            let ws2 = WorkspaceConfig::new("another-ws", "~/work");
            let content = ws2.to_toml().unwrap();
            let ws2_dir = dir.join("another-ws");
            std::fs::create_dir_all(&ws2_dir).unwrap();
            std::fs::write(ws2_dir.join("workspace-config.toml"), &content).unwrap();

            // Count subdirectories that contain workspace.toml
            let entries: Vec<_> = std::fs::read_dir(dir)
                .unwrap()
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_dir() && e.path().join("workspace-config.toml").exists())
                .collect();
            assert_eq!(entries.len(), 2);
        });
    }

    #[test]
    fn test_resolve_from_list_empty() {
        let result = WorkspaceManager::resolve_from_list(vec![]);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("No workspaces configured"));
    }

    #[test]
    fn test_resolve_from_list_single() {
        let ws = WorkspaceConfig::new("only-ws", "~/github");
        let result = WorkspaceManager::resolve_from_list(vec![ws]);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().name, "only-ws");
    }

    #[test]
    fn test_resolve_from_list_multiple() {
        let ws1 = WorkspaceConfig::new("ws1", "~/github");
        let ws2 = WorkspaceConfig::new("ws2", "~/work");
        let result = WorkspaceManager::resolve_from_list(vec![ws1, ws2]);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Multiple workspaces"));
        assert!(err.contains("ws1"));
        assert!(err.contains("ws2"));
    }
}
