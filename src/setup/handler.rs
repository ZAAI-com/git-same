//! Setup wizard event handling.

use super::state::{tilde_collapse, AuthStatus, OrgEntry, SetupOutcome, SetupState, SetupStep};
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
    if state.path_suggestions_mode {
        handle_path_suggestions(state, key);
    } else {
        handle_path_input(state, key);
    }
}

fn confirm_path(state: &mut SetupState) {
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

fn handle_path_suggestions(state: &mut SetupState, key: KeyEvent) {
    match key.code {
        KeyCode::Up => {
            if state.path_suggestion_index > 0 {
                state.path_suggestion_index -= 1;
            }
        }
        KeyCode::Down => {
            if state.path_suggestion_index + 1 < state.path_suggestions.len() {
                state.path_suggestion_index += 1;
            }
        }
        KeyCode::Enter => {
            if let Some(s) = state.path_suggestions.get(state.path_suggestion_index) {
                state.base_path = s.path.clone();
                state.path_cursor = state.base_path.len();
            }
            confirm_path(state);
        }
        KeyCode::Tab => {
            if let Some(s) = state.path_suggestions.get(state.path_suggestion_index) {
                state.base_path = s.path.clone();
                state.path_cursor = state.base_path.len();
            }
            state.path_suggestions_mode = false;
            state.path_completions = compute_completions(&state.base_path);
            state.path_completion_index = 0;
        }
        KeyCode::Esc => {
            state.prev_step();
        }
        KeyCode::Backspace => {
            state.path_suggestions_mode = false;
            if state.path_cursor > 0 {
                state.path_cursor -= 1;
                state.base_path.remove(state.path_cursor);
            }
            state.path_completions = compute_completions(&state.base_path);
            state.path_completion_index = 0;
        }
        KeyCode::Char(c) => {
            state.path_suggestions_mode = false;
            state.base_path.clear();
            state.base_path.push(c);
            state.path_cursor = 1;
            state.path_completions = compute_completions(&state.base_path);
            state.path_completion_index = 0;
        }
        _ => {}
    }
}

fn handle_path_input(state: &mut SetupState, key: KeyEvent) {
    match key.code {
        KeyCode::Tab => {
            apply_tab_completion(state);
        }
        KeyCode::Enter => {
            confirm_path(state);
        }
        KeyCode::Esc => {
            state.prev_step();
        }
        KeyCode::Backspace => {
            if state.path_cursor > 0 {
                state.path_cursor -= 1;
                state.base_path.remove(state.path_cursor);
                state.path_completions = compute_completions(&state.base_path);
                state.path_completion_index = 0;
            }
        }
        KeyCode::Delete => {
            if state.path_cursor < state.base_path.len() {
                state.base_path.remove(state.path_cursor);
                state.path_completions = compute_completions(&state.base_path);
                state.path_completion_index = 0;
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
            state.path_completions = compute_completions(&state.base_path);
            state.path_completion_index = 0;
        }
        _ => {}
    }
}

/// Compute directory completions for the current input path.
fn compute_completions(input: &str) -> Vec<String> {
    if input.is_empty() {
        return Vec::new();
    }
    let expanded = shellexpand::tilde(input);
    let path = std::path::Path::new(expanded.as_ref());

    let (parent, prefix) = if expanded.ends_with('/') {
        (path.to_path_buf(), String::new())
    } else {
        let parent = path.parent().unwrap_or(path).to_path_buf();
        let prefix = path
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_default();
        (parent, prefix)
    };

    let mut results = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&parent) {
        for entry in entries.flatten() {
            if !entry.path().is_dir() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.') {
                continue;
            }
            if prefix.is_empty() || name.starts_with(&prefix) {
                let full = parent.join(&name);
                let display = tilde_collapse(&full.to_string_lossy());
                results.push(format!("{}/", display));
            }
        }
    }
    results.sort();
    results
}

fn apply_tab_completion(state: &mut SetupState) {
    if state.path_completions.is_empty() {
        return;
    }
    if state.path_completions.len() == 1 {
        state.base_path = state.path_completions[0].clone();
        state.path_cursor = state.base_path.len();
        state.path_completions = compute_completions(&state.base_path);
        state.path_completion_index = 0;
    } else {
        let common = longest_common_prefix(&state.path_completions);
        if common.len() > state.base_path.len() {
            state.base_path = common;
            state.path_cursor = state.base_path.len();
            state.path_completions = compute_completions(&state.base_path);
            state.path_completion_index = 0;
        } else {
            // Already at common prefix, cycle through completions
            state.base_path = state.path_completions[state.path_completion_index].clone();
            state.path_cursor = state.base_path.len();
            state.path_completion_index =
                (state.path_completion_index + 1) % state.path_completions.len();
        }
    }
}

fn longest_common_prefix(strings: &[String]) -> String {
    if strings.is_empty() {
        return String::new();
    }
    let first = &strings[0];
    let mut len = first.len();
    for s in &strings[1..] {
        len = len.min(s.len());
        for (i, (a, b)) in first.bytes().zip(s.bytes()).enumerate() {
            if a != b {
                len = len.min(i);
                break;
            }
        }
    }
    first[..len].to_string()
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
