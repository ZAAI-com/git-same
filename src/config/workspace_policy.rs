//! Workspace resolution rules (policy concern only).

use super::parser::Config;
use super::workspace::tilde_collapse_path;
use super::workspace::WorkspaceConfig;
use super::workspace_store::WorkspaceStore;
use crate::errors::AppError;
use std::path::Path;

/// Workspace policy helpers.
pub struct WorkspacePolicy;

impl WorkspacePolicy {
    /// Walk up from `start` to find the nearest `.git-same/config.toml`.
    ///
    /// Returns the workspace root (parent of `.git-same/`) if found.
    pub fn detect_from_cwd(start: &Path) -> Option<std::path::PathBuf> {
        let mut current = start.to_path_buf();
        loop {
            let config = WorkspaceStore::config_path(&current);
            if config.exists() {
                return Some(current);
            }
            if !current.pop() {
                break;
            }
        }
        None
    }

    /// Resolve which workspace to use.
    ///
    /// Priority:
    /// 1. Explicit `--workspace <path|name>` argument
    /// 2. CWD auto-detection (walk up looking for `.git-same/`)
    /// 3. Global `default_workspace` path
    /// 4. Single-workspace auto-select
    /// 5. Error
    pub fn resolve(name: Option<&str>, config: &Config) -> Result<WorkspaceConfig, AppError> {
        // 1. Explicit selector (path or unique folder name)
        if let Some(value) = name {
            let expanded = shellexpand::tilde(value);
            let root = Path::new(expanded.as_ref());
            match WorkspaceStore::load(root) {
                Ok(ws) => return Ok(ws),
                Err(path_err) => {
                    let workspaces = WorkspaceStore::list()?;
                    return Self::resolve_selector_from_list(value, workspaces).map_err(|_| {
                        if Self::looks_like_path(value) {
                            path_err
                        } else {
                            AppError::config(format!(
                                "No workspace matched selector '{}'. Use 'gisa workspace list' and \
                                 pass a workspace folder name or path.",
                                value
                            ))
                        }
                    });
                }
            }
        }

        // 2. CWD auto-detection
        if let Ok(cwd) = std::env::current_dir() {
            if let Some(root) = Self::detect_from_cwd(&cwd) {
                return WorkspaceStore::load(&root);
            }
        }

        // 3. Global default_workspace
        if let Some(ref default_path) = config.default_workspace {
            let expanded = shellexpand::tilde(default_path);
            let root = Path::new(expanded.as_ref());
            return WorkspaceStore::load(root);
        }

        // 4. Single-workspace auto-select (or error)
        let workspaces = WorkspaceStore::list()?;
        Self::resolve_from_list(workspaces)
    }

    /// Resolve from an already-loaded list of workspaces (no filesystem access).
    pub fn resolve_from_list(
        workspaces: Vec<WorkspaceConfig>,
    ) -> Result<WorkspaceConfig, AppError> {
        match workspaces.len() {
            0 => Err(AppError::config(
                "No workspaces configured. Run 'gisa setup' first.",
            )),
            1 => Ok(workspaces
                .into_iter()
                .next()
                .expect("single workspace exists")),
            _ => {
                let labels: Vec<String> = workspaces.iter().map(|w| w.display_label()).collect();
                Err(AppError::config(format!(
                    "Multiple workspaces configured. Use --workspace <path|name> to select one, \
                     or set a default with 'gisa workspace default <path|name>': {}",
                    labels.join(", ")
                )))
            }
        }
    }

    fn resolve_selector_from_list(
        selector: &str,
        workspaces: Vec<WorkspaceConfig>,
    ) -> Result<WorkspaceConfig, AppError> {
        let matches: Vec<WorkspaceConfig> = workspaces
            .into_iter()
            .filter(|ws| {
                let folder_name_matches = ws
                    .root_path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .map(|name| name == selector)
                    .unwrap_or(false);
                let path_matches = ws.root_path.to_string_lossy() == selector;
                let tilde_path_matches = tilde_collapse_path(&ws.root_path) == selector;
                folder_name_matches || path_matches || tilde_path_matches
            })
            .collect();

        match matches.len() {
            1 => Ok(matches
                .into_iter()
                .next()
                .expect("single selector match exists")),
            0 => Err(AppError::config("No workspace matched selector")),
            _ => {
                let labels: Vec<String> = matches.iter().map(|ws| ws.display_label()).collect();
                Err(AppError::config(format!(
                    "Workspace selector '{}' is ambiguous. Use an explicit path instead: {}",
                    selector,
                    labels.join(", ")
                )))
            }
        }
    }

    fn looks_like_path(value: &str) -> bool {
        value.contains(std::path::MAIN_SEPARATOR)
            || value.starts_with('.')
            || value.starts_with('~')
    }
}

#[cfg(test)]
#[path = "workspace_policy_tests.rs"]
mod tests;
