//! Workspace manager facade.
//!
//! Keeps a stable API while delegating to `WorkspaceStore` and `WorkspacePolicy`.

use super::workspace::WorkspaceConfig;
use super::{workspace_policy::WorkspacePolicy, workspace_store::WorkspaceStore};
use crate::errors::AppError;
use std::path::{Path, PathBuf};

/// Compatibility facade for workspace operations.
pub struct WorkspaceManager;

impl WorkspaceManager {
    /// List all workspace configs.
    pub fn list() -> Result<Vec<WorkspaceConfig>, AppError> {
        WorkspaceStore::list()
    }

    /// Load a specific workspace by root path.
    pub fn load(root: &Path) -> Result<WorkspaceConfig, AppError> {
        WorkspaceStore::load(root)
    }

    /// Save a workspace config (create or update).
    pub fn save(workspace: &WorkspaceConfig) -> Result<(), AppError> {
        WorkspaceStore::save(workspace)
    }

    /// Delete a workspace by root path.
    pub fn delete(root: &Path) -> Result<(), AppError> {
        WorkspaceStore::delete(root)
    }

    /// Returns the `.git-same/` directory for a workspace root.
    pub fn dot_dir(root: &Path) -> PathBuf {
        WorkspaceStore::dot_dir(root)
    }

    /// Returns the cache file path for a workspace root.
    pub fn cache_path(root: &Path) -> PathBuf {
        WorkspaceStore::cache_path(root)
    }

    /// Returns the sync history file path for a workspace root.
    pub fn sync_history_path(root: &Path) -> PathBuf {
        WorkspaceStore::sync_history_path(root)
    }

    /// Walk up from `start` to find the nearest `.git-same/config.toml`.
    pub fn detect_from_cwd(start: &Path) -> Option<PathBuf> {
        WorkspacePolicy::detect_from_cwd(start)
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
}

#[cfg(test)]
#[path = "workspace_manager_tests.rs"]
mod tests;
