//! Setup wizard event handling.

use super::state::{AuthStatus, OrgEntry, SetupOutcome, SetupState, SetupStep};
use crate::auth::{get_auth_for_provider, gh_cli};
use crate::config::{WorkspaceConfig, WorkspaceManager};
use crate::provider::{create_provider, Credentials};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Handle a key event in the setup wizard.
///
/// Returns true if the event triggered an async operation that should be awaited.
pub async fn handle_key(state: &mut SetupState, key: KeyEvent) {
    // Global: Ctrl+C quits
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        state.outcome = Some(SetupOutcome::Cancelled);
        state.should_quit = true;
        return;
    }

    match state.step {
        SetupStep::SelectProvider => handle_provider(state, key),
        SetupStep::Authenticate => handle_auth(state, key).await,
        SetupStep::SelectPath => handle_path(state, key),
        SetupStep::SelectOrgs => handle_orgs(state, key).await,
        SetupStep::Confirm => handle_confirm(state, key),
    }
}

fn handle_provider(state: &mut SetupState, key: KeyEvent) {
    match key.code {
        KeyCode::Up | KeyCode::Char('k') => {
            if state.provider_index > 0 {
                state.provider_index -= 1;
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if state.provider_index + 1 < state.provider_choices.len() {
                state.provider_index += 1;
            }
        }
        KeyCode::Enter => {
            if state.provider_choices[state.provider_index].available {
                state.auth_status = AuthStatus::Pending;
                state.next_step();
            }
        }
        KeyCode::Esc => {
            state.prev_step();
        }
        _ => {}
    }
}

async fn handle_auth(state: &mut SetupState, key: KeyEvent) {
    match key.code {
        KeyCode::Enter => {
            match &state.auth_status {
                AuthStatus::Pending | AuthStatus::Failed(_) => {
                    // Attempt authentication
                    state.auth_status = AuthStatus::Checking;
                    do_authenticate(state).await;
                }
                AuthStatus::Success => {
                    state.next_step();
                }
                AuthStatus::Checking => {}
            }
        }
        KeyCode::Esc => {
            state.prev_step();
        }
        _ => {}
    }
}

async fn do_authenticate(state: &mut SetupState) {
    let provider_entry = state.build_workspace_provider().to_provider_entry();
    match get_auth_for_provider(&provider_entry) {
        Ok(auth) => {
            let username = auth.username.or_else(|| gh_cli::get_username().ok());
            state.username = username;
            state.auth_token = Some(auth.token);
            state.auth_status = AuthStatus::Success;
        }
        Err(e) => {
            state.auth_status = AuthStatus::Failed(e.to_string());
        }
    }
}

fn handle_path(state: &mut SetupState, key: KeyEvent) {
    match key.code {
        KeyCode::Enter => {
            if state.base_path.is_empty() {
                state.error_message = Some("Base path cannot be empty".to_string());
            } else {
                state.error_message = None;
                state.org_loading = true;
                state.orgs.clear();
                state.org_error = None;
                state.next_step();
            }
        }
        KeyCode::Esc => {
            state.prev_step();
        }
        KeyCode::Backspace => {
            if state.path_cursor > 0 {
                state.path_cursor -= 1;
                state.base_path.remove(state.path_cursor);
            }
        }
        KeyCode::Delete => {
            if state.path_cursor < state.base_path.len() {
                state.base_path.remove(state.path_cursor);
            }
        }
        KeyCode::Left => {
            if state.path_cursor > 0 {
                state.path_cursor -= 1;
            }
        }
        KeyCode::Right => {
            if state.path_cursor < state.base_path.len() {
                state.path_cursor += 1;
            }
        }
        KeyCode::Home => {
            state.path_cursor = 0;
        }
        KeyCode::End => {
            state.path_cursor = state.base_path.len();
        }
        KeyCode::Char(c) => {
            state.base_path.insert(state.path_cursor, c);
            state.path_cursor += 1;
        }
        _ => {}
    }
}

async fn handle_orgs(state: &mut SetupState, key: KeyEvent) {
    if state.org_loading {
        // Trigger org discovery
        do_discover_orgs(state).await;
        return;
    }

    match key.code {
        KeyCode::Up | KeyCode::Char('k') => {
            if state.org_index > 0 {
                state.org_index -= 1;
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if state.org_index + 1 < state.orgs.len() {
                state.org_index += 1;
            }
        }
        KeyCode::Char(' ') => {
            if !state.orgs.is_empty() {
                state.orgs[state.org_index].selected = !state.orgs[state.org_index].selected;
            }
        }
        KeyCode::Char('a') => {
            for org in &mut state.orgs {
                org.selected = true;
            }
        }
        KeyCode::Char('n') => {
            for org in &mut state.orgs {
                org.selected = false;
            }
        }
        KeyCode::Enter => {
            if state.org_error.is_some() {
                // Retry
                state.org_loading = true;
                state.org_error = None;
            } else {
                state.next_step();
            }
        }
        KeyCode::Esc => {
            state.prev_step();
        }
        _ => {}
    }
}

async fn do_discover_orgs(state: &mut SetupState) {
    let Some(ref token) = state.auth_token else {
        state.org_error = Some("Not authenticated".to_string());
        state.org_loading = false;
        return;
    };

    let provider_entry = state.build_workspace_provider().to_provider_entry();
    let api_url = provider_entry.effective_api_url();

    let credentials = Credentials {
        token: token.clone(),
        api_base_url: api_url,
        username: state.username.clone(),
    };

    match create_provider(&provider_entry, &credentials.token) {
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
                state.orgs = org_entries;
                state.org_index = 0;
                state.org_loading = false;
            }
            Err(e) => {
                state.org_error = Some(e.to_string());
                state.org_loading = false;
            }
        },
        Err(e) => {
            state.org_error = Some(e.to_string());
            state.org_loading = false;
        }
    }
}

fn handle_confirm(state: &mut SetupState, key: KeyEvent) {
    match key.code {
        KeyCode::Enter => {
            // Save workspace config
            match save_workspace(state) {
                Ok(()) => {
                    state.next_step(); // Triggers Completed + should_quit
                }
                Err(e) => {
                    state.error_message = Some(e.to_string());
                }
            }
        }
        KeyCode::Esc => {
            state.prev_step();
        }
        _ => {}
    }
}

fn save_workspace(state: &SetupState) -> Result<(), crate::errors::AppError> {
    let mut ws = WorkspaceConfig::new(&state.workspace_name, &state.base_path);
    ws.provider = state.build_workspace_provider();
    ws.username = state.username.clone().unwrap_or_default();
    ws.orgs = state.selected_orgs();

    WorkspaceManager::save(&ws)?;
    Ok(())
}
