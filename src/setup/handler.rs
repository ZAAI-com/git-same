//! Setup wizard event handling.

use super::state::{
    tilde_collapse, AuthStatus, OrgEntry, PathBrowseEntry, SetupOutcome, SetupState, SetupStep,
};
use crate::auth::{get_auth_for_provider, gh_cli};
use crate::config::{WorkspaceConfig, WorkspaceManager};
use crate::provider::{create_provider, Credentials};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Handle a key event in the setup wizard.
///
/// Returns true if the event triggered an async operation that should be awaited.
pub async fn handle_key(state: &mut SetupState, key: KeyEvent) {
    // Global quit shortcuts
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        state.outcome = Some(SetupOutcome::Cancelled);
        state.should_quit = true;
        return;
    }
    if key.modifiers == KeyModifiers::NONE
        && key.code == KeyCode::Char('q')
        && !matches!(state.step, SetupStep::SelectPath)
    {
        state.outcome = Some(SetupOutcome::Cancelled);
        state.should_quit = true;
        return;
    }
    if key.modifiers == KeyModifiers::NONE && key.code == KeyCode::Esc {
        state.outcome = Some(SetupOutcome::Cancelled);
        state.should_quit = true;
        return;
    }
    if key.modifiers == KeyModifiers::NONE {
        match key.code {
            KeyCode::Left => {
                state.prev_step();
                return;
            }
            KeyCode::Right => {
                handle_step_forward(state).await;
                return;
            }
            _ => {}
        }
    }

    match state.step {
        SetupStep::Welcome => handle_welcome(state, key),
        SetupStep::SelectProvider => handle_provider(state, key),
        SetupStep::Authenticate => handle_auth(state, key).await,
        SetupStep::SelectPath => handle_path(state, key),
        SetupStep::SelectOrgs => handle_orgs(state, key).await,
        SetupStep::Confirm => handle_confirm(state, key),
        SetupStep::Complete => handle_complete(state, key),
    }
}

async fn handle_step_forward(state: &mut SetupState) {
    match state.step {
        SetupStep::Welcome => {
            state.next_step();
        }
        SetupStep::SelectProvider => {
            if state.provider_choices[state.provider_index].available {
                state.auth_status = AuthStatus::Pending;
                state.next_step();
            }
        }
        SetupStep::Authenticate => match state.auth_status.clone() {
            AuthStatus::Pending | AuthStatus::Failed(_) => {
                state.auth_status = AuthStatus::Checking;
                do_authenticate(state).await;
            }
            AuthStatus::Success => {
                state.next_step();
            }
            AuthStatus::Checking => {}
        },
        SetupStep::SelectOrgs => {
            if state.org_loading {
                do_discover_orgs(state).await;
            } else if state.org_error.is_some() {
                state.org_loading = true;
                state.org_error = None;
            } else {
                state.next_step();
            }
        }
        SetupStep::SelectPath => {
            if state.path_browse_mode {
                if !state.path_browse_current_dir.is_empty() {
                    state.base_path = state.path_browse_current_dir.clone();
                    state.path_cursor = state.base_path.len();
                }
                close_path_browse_to_input(state);
            } else if state.path_suggestions_mode {
                if let Some(s) = state.path_suggestions.get(state.path_suggestion_index) {
                    state.base_path = s.path.clone();
                    state.path_cursor = state.base_path.len();
                }
            }
            confirm_path(state);
        }
        SetupStep::Confirm => match save_workspace(state) {
            Ok(()) => {
                state.next_step();
            }
            Err(e) => {
                state.error_message = Some(e.to_string());
            }
        },
        SetupStep::Complete => {
            state.next_step();
        }
    }
}

fn handle_welcome(state: &mut SetupState, key: KeyEvent) {
    match key.code {
        KeyCode::Enter => {
            state.next_step();
        }
        KeyCode::Esc => {
            state.prev_step();
        }
        _ => {}
    }
}

fn handle_provider(state: &mut SetupState, key: KeyEvent) {
    match key.code {
        KeyCode::Up => {
            if state.provider_index > 0 {
                state.provider_index -= 1;
            }
        }
        KeyCode::Down => {
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
    if state.path_browse_mode {
        handle_path_browse(state, key);
    } else if state.path_suggestions_mode {
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
        state.next_step();
    }
}

fn open_path_browse_mode(state: &mut SetupState, seed_path: &str) {
    let dir = resolve_browse_seed(seed_path);
    state.path_browse_info = None;
    set_browse_directory(state, dir);
    state.path_browse_mode = true;
}

fn resolve_browse_seed(seed_path: &str) -> std::path::PathBuf {
    if !seed_path.is_empty() {
        let expanded = shellexpand::tilde(seed_path);
        let candidate = std::path::PathBuf::from(expanded.as_ref());
        if candidate.is_dir() {
            return candidate;
        }
        if let Some(parent) = candidate.parent() {
            if parent.is_dir() {
                return parent.to_path_buf();
            }
        }
    }

    std::env::current_dir()
        .or_else(|_| std::env::var("HOME").map(std::path::PathBuf::from))
        .unwrap_or_else(|_| std::path::PathBuf::from("/"))
}

fn set_browse_directory(state: &mut SetupState, dir: std::path::PathBuf) {
    state.path_browse_current_dir = tilde_collapse(&dir.to_string_lossy());
    let (entries, browse_error) = read_browse_entries(&dir, state.path_browse_show_hidden);
    state.path_browse_entries = entries;
    state.path_browse_error = browse_error;
    state.path_browse_index = 0;
}

fn read_browse_entries(
    dir: &std::path::Path,
    show_hidden: bool,
) -> (Vec<PathBrowseEntry>, Option<String>) {
    let mut entries = Vec::new();
    let mut browse_error = None;

    if let Some(parent) = dir.parent() {
        entries.push(PathBrowseEntry {
            label: ".. (parent)".to_string(),
            path: tilde_collapse(&parent.to_string_lossy()),
        });
    }

    let mut children = Vec::new();
    match std::fs::read_dir(dir) {
        Ok(dir_entries) => {
            for entry_result in dir_entries {
                match entry_result {
                    Ok(entry) => {
                        let path = entry.path();
                        if !path.is_dir() {
                            continue;
                        }
                        let name = entry.file_name().to_string_lossy().to_string();
                        if !show_hidden && name.starts_with('.') {
                            continue;
                        }
                        children.push(PathBrowseEntry {
                            label: format!("{name}/"),
                            path: tilde_collapse(&path.to_string_lossy()),
                        });
                    }
                    Err(e) => {
                        if browse_error.is_none() {
                            browse_error = Some(format!("Some entries could not be read: {e}"));
                        }
                    }
                }
            }
        }
        Err(e) => {
            browse_error = Some(format!(
                "Cannot read '{}': {e}",
                tilde_collapse(&dir.to_string_lossy())
            ));
        }
    }
    children.sort_by_key(|entry| entry.label.to_lowercase());
    entries.extend(children);
    (entries, browse_error)
}

fn close_path_browse_to_input(state: &mut SetupState) {
    state.path_browse_mode = false;
    state.path_suggestions_mode = false;
    state.path_browse_error = None;
    state.path_browse_info = None;
    state.path_cursor = state.base_path.len();
    state.path_completions = compute_completions(&state.base_path);
    state.path_completion_index = 0;
}

fn current_browse_dir(state: &SetupState) -> Option<std::path::PathBuf> {
    if state.path_browse_current_dir.is_empty() {
        return None;
    }
    let expanded = shellexpand::tilde(&state.path_browse_current_dir);
    let dir = std::path::PathBuf::from(expanded.as_ref());
    if dir.is_dir() {
        Some(dir)
    } else {
        None
    }
}

fn open_selected_browse_entry(state: &mut SetupState) {
    if let Some(path) = state
        .path_browse_entries
        .get(state.path_browse_index)
        .map(|entry| entry.path.clone())
    {
        let expanded = shellexpand::tilde(&path);
        let dir = std::path::PathBuf::from(expanded.as_ref());
        if dir.is_dir() {
            state.path_browse_info = None;
            set_browse_directory(state, dir);
        } else {
            state.path_browse_error = Some(format!("Directory no longer exists: {path}"));
        }
    }
}

fn use_current_browse_folder(state: &mut SetupState) {
    if !state.path_browse_current_dir.is_empty() {
        state.base_path = state.path_browse_current_dir.clone();
        state.path_cursor = state.base_path.len();
        close_path_browse_to_input(state);
    }
}

fn jump_to_home_directory(state: &mut SetupState) {
    match std::env::var("HOME") {
        Ok(home) => {
            let dir = std::path::PathBuf::from(home);
            if dir.is_dir() {
                state.path_browse_info = Some("Jumped to home directory".to_string());
                set_browse_directory(state, dir);
            } else {
                state.path_browse_error = Some("Home directory is not accessible".to_string());
            }
        }
        Err(_) => {
            state.path_browse_error = Some("HOME environment variable is not set".to_string());
        }
    }
}

fn jump_to_current_directory(state: &mut SetupState) {
    match std::env::current_dir() {
        Ok(dir) => {
            state.path_browse_info = Some("Jumped to current directory".to_string());
            set_browse_directory(state, dir);
        }
        Err(e) => {
            state.path_browse_error = Some(format!("Cannot read current directory: {e}"));
        }
    }
}

fn jump_to_root_directory(state: &mut SetupState) {
    let Some(current) = current_browse_dir(state) else {
        state.path_browse_error = Some("Cannot resolve current browse directory".to_string());
        return;
    };
    let root = current
        .ancestors()
        .last()
        .unwrap_or(current.as_path())
        .to_path_buf();
    state.path_browse_info = Some("Jumped to filesystem root".to_string());
    set_browse_directory(state, root);
}

fn toggle_hidden_directories(state: &mut SetupState) {
    state.path_browse_show_hidden = !state.path_browse_show_hidden;
    let message = if state.path_browse_show_hidden {
        "Showing hidden folders"
    } else {
        "Hiding hidden folders"
    };

    if let Some(current) = current_browse_dir(state) {
        set_browse_directory(state, current);
        state.path_browse_info = Some(message.to_string());
    } else {
        state.path_browse_error = Some("Cannot refresh browse list".to_string());
    }
}

fn create_folder_in_current_directory(state: &mut SetupState) {
    let Some(current) = current_browse_dir(state) else {
        state.path_browse_error = Some("Cannot resolve current browse directory".to_string());
        return;
    };

    let mut selected_path = None;
    for idx in 1..=999 {
        let name = if idx == 1 {
            "new-folder".to_string()
        } else {
            format!("new-folder-{idx}")
        };
        let candidate = current.join(&name);
        if !candidate.exists() {
            match std::fs::create_dir(&candidate) {
                Ok(()) => {
                    selected_path = Some(tilde_collapse(&candidate.to_string_lossy()));
                    state.path_browse_info = Some(format!("Created '{name}'"));
                    state.path_browse_error = None;
                }
                Err(e) => {
                    state.path_browse_error = Some(format!("Cannot create folder: {e}"));
                }
            }
            break;
        }
    }

    set_browse_directory(state, current);
    if let Some(path) = selected_path {
        if let Some(index) = state
            .path_browse_entries
            .iter()
            .position(|entry| entry.path == path)
        {
            state.path_browse_index = index;
        }
    } else if state.path_browse_error.is_none() {
        state.path_browse_error = Some("Could not allocate a new folder name".to_string());
    }
}

fn handle_path_browse(state: &mut SetupState, key: KeyEvent) {
    match key.code {
        KeyCode::Up => {
            if state.path_browse_index > 0 {
                state.path_browse_index -= 1;
            }
        }
        KeyCode::Down => {
            if state.path_browse_index + 1 < state.path_browse_entries.len() {
                state.path_browse_index += 1;
            }
        }
        KeyCode::Right | KeyCode::Enter => {
            open_selected_browse_entry(state);
        }
        KeyCode::Left => {
            if let Some(current) = current_browse_dir(state) {
                if let Some(parent) = current.parent() {
                    if parent.is_dir() {
                        state.path_browse_info = None;
                        set_browse_directory(state, parent.to_path_buf());
                    }
                }
            }
        }
        KeyCode::Char('u') => {
            use_current_browse_folder(state);
        }
        KeyCode::Char('h') => {
            jump_to_home_directory(state);
        }
        KeyCode::Char('c') => {
            jump_to_current_directory(state);
        }
        KeyCode::Char('r') => {
            jump_to_root_directory(state);
        }
        KeyCode::Char('.') => {
            toggle_hidden_directories(state);
        }
        KeyCode::Char('n') => {
            create_folder_in_current_directory(state);
        }
        KeyCode::Esc => {
            close_path_browse_to_input(state);
        }
        _ => {}
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
        KeyCode::Char('b') => {
            if let Some(s) = state.path_suggestions.get(state.path_suggestion_index) {
                state.base_path = s.path.clone();
                state.path_cursor = state.base_path.len();
            }
            let seed = state.base_path.clone();
            open_path_browse_mode(state, &seed);
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
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('b') {
        let seed = state.base_path.clone();
        open_path_browse_mode(state, &seed);
        return;
    }

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
        let parent = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .map(std::path::Path::to_path_buf)
            .unwrap_or_else(|| {
                std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."))
            });
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
        KeyCode::Up => {
            if state.org_index > 0 {
                state.org_index -= 1;
            }
        }
        KeyCode::Down => {
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
            // Save workspace config and advance to Complete screen
            match save_workspace(state) {
                Ok(()) => {
                    state.next_step();
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

fn handle_complete(state: &mut SetupState, key: KeyEvent) {
    match key.code {
        KeyCode::Enter | KeyCode::Char('s') => {
            state.next_step(); // Triggers Completed + should_quit
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

#[cfg(test)]
#[path = "handler_tests.rs"]
mod tests;
