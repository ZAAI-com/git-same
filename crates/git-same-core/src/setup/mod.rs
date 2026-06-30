//! Headless setup wizard state and services.
//!
//! UI hosts own input mapping and rendering, while this module owns the shared
//! setup state machine plus side-effecting setup services.

pub mod state;

use crate::auth::{get_auth_for_provider, gh_cli};
use crate::checks::CheckResult;
use crate::config::{Config, WorkspaceConfig, WorkspaceManager, WorkspaceProvider};
use crate::errors::{AppError, Result};
use crate::provider::create_provider;
pub use state::{
    tilde_collapse, AuthStatus, OrgEntry, PathBrowseEntry, PathSuggestion, ProviderChoice,
    SetupOutcome, SetupState, SetupStep,
};

/// Authentication result for setup hosts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetupAuthResult {
    /// Token returned by the configured provider auth flow.
    pub token: String,
    /// Authenticated username when it can be detected.
    pub username: Option<String>,
}

/// Marks the requirements step as loading and records the config path.
pub fn maybe_start_requirements_checks(state: &mut SetupState) -> bool {
    if state.step != SetupStep::Requirements || state.checks_triggered {
        return false;
    }

    state.checks_triggered = true;
    state.checks_loading = true;
    state.config_path_display = Config::default_path().ok().map(|p| p.display().to_string());
    true
}

/// Applies completed requirements checks to setup state.
pub fn apply_requirements_check_results(state: &mut SetupState, results: Vec<CheckResult>) {
    state.check_results = results;
    state.checks_loading = false;
}

/// Runs system requirements checks and applies the results.
pub async fn run_requirements_checks(state: &mut SetupState) {
    let results = crate::checks::check_requirements().await;
    apply_requirements_check_results(state, results);
}

/// Authenticates the selected provider and returns token plus username details.
pub async fn authenticate_provider(
    ws_provider: WorkspaceProvider,
) -> std::result::Result<SetupAuthResult, String> {
    let result = tokio::task::spawn_blocking(move || match get_auth_for_provider(&ws_provider) {
        Ok(auth) => {
            let username = auth.username.or_else(|| gh_cli::get_username().ok());
            Ok(SetupAuthResult {
                token: auth.token,
                username,
            })
        }
        Err(e) => Err(e.to_string()),
    })
    .await;

    match result {
        Ok(result) => result,
        Err(e) => Err(format!("Auth task failed: {}", e)),
    }
}

/// Discovers selectable organization entries for setup.
pub async fn discover_org_entries(
    ws_provider: WorkspaceProvider,
    token: String,
) -> std::result::Result<Vec<OrgEntry>, String> {
    match create_provider(&ws_provider, &token) {
        Ok(provider) => match provider.get_organizations().await {
            Ok(orgs) => {
                let mut org_entries: Vec<OrgEntry> = Vec::new();
                for org in &orgs {
                    let repo_count = provider
                        .get_org_repos(&org.login)
                        .await
                        .map(|r| r.len())
                        .unwrap_or(0);
                    org_entries.push(OrgEntry {
                        name: org.login.clone(),
                        repo_count,
                        selected: true,
                    });
                }
                org_entries.sort_by(|a, b| a.name.cmp(&b.name));
                Ok(org_entries)
            }
            Err(e) => Err(e.to_string()),
        },
        Err(e) => Err(e.to_string()),
    }
}

/// Persists the workspace represented by setup state.
pub fn save_workspace(state: &SetupState) -> Result<()> {
    let expanded = shellexpand::tilde(&state.base_path);
    let root = std::path::Path::new(expanded.as_ref());
    std::fs::create_dir_all(root).map_err(|e| {
        AppError::config(format!(
            "Failed to create workspace directory '{}': {}",
            root.display(),
            e
        ))
    })?;
    let root = std::fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());

    let mut ws = WorkspaceConfig::new_from_root(&root);
    ws.provider = state.build_workspace_provider();
    ws.username = state.username.clone().unwrap_or_default();
    ws.orgs = state.selected_orgs();

    WorkspaceManager::save(&ws)?;

    // Paint the Git-Same workspace folder icon onto the root so Finder marks
    // it the way Synology Drive marks its synced folders. Failure is logged
    // but non-fatal — the workspace is already persisted at this point and
    // we don't want a Cocoa-level glitch to abort setup. Opt-out via
    // `[ui] custom_folder_icon = false` in the global config.
    let ui = Config::load().map(|c| c.ui).unwrap_or_default();
    if ui.custom_folder_icon {
        crate::macos::folder_icon::set_or_log(
            &root,
            crate::macos::folder_icon::WORKSPACE_FOLDER_ICNS,
        );
    }

    Ok(())
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
