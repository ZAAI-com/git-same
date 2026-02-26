//! Workspace manager facade.
//!
//! This compatibility layer keeps the existing `WorkspaceManager` API stable
//! while delegating storage and policy responsibilities to dedicated modules.

use super::workspace::WorkspaceConfig;
use super::{workspace_policy::WorkspacePolicy, workspace_store::WorkspaceStore};
use crate::errors::AppError;
use crate::types::ProviderKind;
use std::path::{Path, PathBuf};

/// Compatibility facade for workspace operations.
pub struct WorkspaceManager;

impl WorkspaceManager {
    /// Returns the config directory: `~/.config/git-same/`.
    pub fn config_dir() -> Result<PathBuf, AppError> {
        WorkspaceStore::config_dir()
    }

    /// List all workspace configs.
    pub fn list() -> Result<Vec<WorkspaceConfig>, AppError> {
        WorkspaceStore::list()
    }

    /// Load a specific workspace by name.
    pub fn load(name: &str) -> Result<WorkspaceConfig, AppError> {
        WorkspaceStore::load(name)
    }

    /// Save a workspace config (create or update).
    pub fn save(workspace: &WorkspaceConfig) -> Result<(), AppError> {
        WorkspaceStore::save(workspace)
    }

    /// Delete a workspace by name.
    pub fn delete(name: &str) -> Result<(), AppError> {
        WorkspaceStore::delete(name)
    }

    /// Find a workspace whose base_path matches the given directory.
    pub fn find_by_path(path: &Path) -> Result<Option<WorkspaceConfig>, AppError> {
        WorkspaceStore::find_by_path(path)
    }

    /// Load a workspace by its base_path string.
    pub fn load_by_path(path_str: &str) -> Result<WorkspaceConfig, AppError> {
        WorkspaceStore::load_by_path(path_str)
    }

    /// Derive a workspace name from a base path and provider.
    pub fn name_from_path(path: &Path, provider: ProviderKind) -> String {
        WorkspacePolicy::name_from_path(path, provider)
    }

    /// Return a unique workspace name, appending `-2`, `-3`, etc. on collision.
    pub fn unique_name(base: &str) -> Result<String, AppError> {
        WorkspacePolicy::unique_name(base)
    }

    /// Resolve which workspace to use.
    pub fn resolve(
        name: Option<&str>,
        config: &super::parser::Config,
    ) -> Result<WorkspaceConfig, AppError> {
        WorkspacePolicy::resolve(name, config)
    }

    /// Resolve from an already-loaded list of workspaces (no filesystem access).
    pub fn resolve_from_list(
        workspaces: Vec<WorkspaceConfig>,
    ) -> Result<WorkspaceConfig, AppError> {
        WorkspacePolicy::resolve_from_list(workspaces)
    }

    /// Returns the directory path for a workspace: `~/.config/git-same/<name>/`.
    pub fn workspace_dir(name: &str) -> Result<PathBuf, AppError> {
        WorkspaceStore::workspace_dir(name)
    }

    /// Returns the cache file path for a workspace.
    pub fn cache_path(name: &str) -> Result<PathBuf, AppError> {
        WorkspaceStore::cache_path(name)
    }
}

#[cfg(test)]
#[path = "workspace_manager_tests.rs"]
mod tests;
