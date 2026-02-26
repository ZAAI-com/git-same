//! Workspace resolution and naming rules (policy concern only).

use super::parser::Config;
use super::workspace::WorkspaceConfig;
use super::workspace_store::WorkspaceStore;
use crate::errors::AppError;
use crate::types::ProviderKind;
use std::path::Path;

/// Workspace policy helpers.
pub struct WorkspacePolicy;

impl WorkspacePolicy {
    /// Derive a workspace name from a base path and provider.
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
            ProviderKind::GitLabSelfManaged => "glsm",
            ProviderKind::Codeberg => "codeberg",
            ProviderKind::Bitbucket => "bitbucket",
        };
        format!("{}-{}", prefix, last_component)
            .to_lowercase()
            .replace([' ', '_'], "-")
    }

    /// Return a unique workspace name, appending `-2`, `-3`, etc. on collision.
    pub fn unique_name(base: &str) -> Result<String, AppError> {
        let dir = WorkspaceStore::workspace_dir(base)?;
        if !dir.exists() {
            return Ok(base.to_string());
        }

        for suffix in 2..=100 {
            let candidate = format!("{}-{}", base, suffix);
            let candidate_dir = WorkspaceStore::workspace_dir(&candidate)?;
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
    pub fn resolve(name: Option<&str>, config: &Config) -> Result<WorkspaceConfig, AppError> {
        if let Some(value) = name {
            return WorkspaceStore::load(value).or_else(|_| WorkspaceStore::load_by_path(value));
        }

        if let Some(ref default) = config.default_workspace {
            return WorkspaceStore::load(default);
        }

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
                    "Multiple workspaces configured. Use --workspace <path> to select one, \
                     or set a default with 'gisa workspace default <path>': {}",
                    labels.join(", ")
                )))
            }
        }
    }
}

#[cfg(test)]
#[path = "workspace_policy_tests.rs"]
mod tests;
