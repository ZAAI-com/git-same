//! Setup wizard event handling.

use super::state::{
    tilde_collapse, AuthStatus, OrgEntry, PathBrowseEntry, SetupOutcome, SetupState, SetupStep,
};
use crate::auth::{get_auth_for_provider, gh_cli};
use crate::config::{WorkspaceConfig, WorkspaceManager};
use crate::provider::create_provider;
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
    let path_popup_active = state.step == SetupStep::SelectPath && state.path_browse_mode;
    if path_popup_active && key.modifiers == KeyModifiers::NONE {
        match key.code {
            KeyCode::Up
            | KeyCode::Down
            | KeyCode::Left
            | KeyCode::Right
            | KeyCode::Enter
            | KeyCode::Esc => {
                handle_path(state, key);
                return;
            }
            _ => {}
        }
    }
    if key.modifiers == KeyModifiers::NONE
        && key.code == KeyCode::Char('q')
        && !matches!(state.step, SetupStep::SelectPath | SetupStep::Requirements)
    {
        state.outcome = Some(SetupOutcome::Cancelled);
        state.should_quit = true;
        return;
    }
    if !path_popup_active
        && state.step != SetupStep::SelectPath
        && state.step != SetupStep::Requirements
        && key.modifiers == KeyModifiers::NONE
        && key.code == KeyCode::Esc
    {
        state.outcome = Some(SetupOutcome::Cancelled);
        state.should_quit = true;
        return;
    }
    if !path_popup_active
        && state.step != SetupStep::SelectPath
        && key.modifiers == KeyModifiers::NONE
    {
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
        SetupStep::Requirements => handle_requirements(state, key),
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
        SetupStep::Requirements => {
            if !state.checks_loading && state.requirements_passed() {
                state.next_step();
            }
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
                // Discovery is driven externally while loading.
            } else if state.org_error.is_some() {
                state.org_loading = true;
                state.org_discovery_in_progress = false;
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

fn handle_requirements(state: &mut SetupState, key: KeyEvent) {
    match key.code {
        KeyCode::Enter if !state.checks_loading && state.requirements_passed() => {
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
        KeyCode::Up if state.provider_index > 0 => {
            state.provider_index -= 1;
        }
        KeyCode::Down if state.provider_index + 1 < state.provider_choices.len() => {
            state.provider_index += 1;
        }
        KeyCode::Enter if state.provider_choices[state.provider_index].available => {
            state.auth_status = AuthStatus::Pending;
            state.next_step();
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
    let ws_provider = state.build_workspace_provider();
    // Auth shells out to `gh`; run it on the blocking pool so the TUI event
    // loop keeps rendering instead of freezing on a stalled subprocess.
    let result = tokio::task::spawn_blocking(move || match get_auth_for_provider(&ws_provider) {
        Ok(auth) => {
            let username = auth.username.or_else(|| gh_cli::get_username().ok());
            Ok((auth.token, username))
        }
        Err(e) => Err(e.to_string()),
    })
    .await;

    match result {
        Ok(Ok((token, username))) => {
            state.username = username;
            state.auth_token = Some(token);
            state.auth_status = AuthStatus::Success;
        }
        Ok(Err(e)) => {
            state.auth_status = AuthStatus::Failed(e);
        }
        Err(e) => {
            state.auth_status = AuthStatus::Failed(format!("Auth task failed: {}", e));
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

fn open_path_browse_mode(state: &mut SetupState) {
    let dir = resolve_browse_root();
    state.path_browse_info = None;
    set_browse_root(state, dir);
    state.path_suggestions_mode = false;
    state.path_browse_mode = true;
}

fn resolve_browse_root() -> std::path::PathBuf {
    std::env::current_dir()
        .or_else(|_| std::env::var("HOME").map(std::path::PathBuf::from))
        .unwrap_or_else(|_| std::path::PathBuf::from("/"))
}

fn set_browse_root(state: &mut SetupState, dir: std::path::PathBuf) {
    let root_path = tilde_collapse(&dir.to_string_lossy());
    let (children, browse_error) = read_child_directories(&dir, 1);
    let root = PathBrowseEntry {
        label: browse_label_for_path(&dir),
        path: root_path.clone(),
        depth: 0,
        expanded: true,
        has_children: !children.is_empty(),
    };

    let mut entries = Vec::with_capacity(children.len() + 1);
    entries.push(root);
    entries.extend(children);

    state.path_browse_current_dir = root_path;
    state.path_browse_entries = entries;
    state.path_browse_error = browse_error;
    state.path_browse_index = 0;
}

fn browse_label_for_path(path: &std::path::Path) -> String {
    if path.parent().is_none() {
        "/".to_string()
    } else {
        let name = path
            .file_name()
            .map(|part| part.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string_lossy().to_string());
        format!("{name}/")
    }
}

fn has_visible_child_directory(dir: &std::path::Path) -> bool {
    match std::fs::read_dir(dir) {
        Ok(entries) => entries.flatten().any(|entry| {
            let path = entry.path();
            if !path.is_dir() {
                return false;
            }
            let name = entry.file_name().to_string_lossy().to_string();
            !name.starts_with('.')
        }),
        Err(_) => false,
    }
}

fn read_child_directories(
    dir: &std::path::Path,
    depth: u16,
) -> (Vec<PathBrowseEntry>, Option<String>) {
    let mut children = Vec::new();
    let mut browse_error = None;

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
                        if name.starts_with('.') {
                            continue;
                        }
                        children.push(PathBrowseEntry {
                            label: format!("{name}/"),
                            path: tilde_collapse(&path.to_string_lossy()),
                            depth,
                            expanded: false,
                            has_children: has_visible_child_directory(&path),
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
    (children, browse_error)
}

fn close_path_browse_to_input(state: &mut SetupState) {
    state.path_browse_mode = false;
    state.path_suggestions_mode = false;
    state.path_browse_error = None;
    state.path_browse_info = None;
    state.path_cursor = state.base_path.len();
    state.path_completions.clear();
    state.path_completion_index = 0;
}

fn sync_browse_current_dir(state: &mut SetupState) {
    if let Some(entry) = state.path_browse_entries.get(state.path_browse_index) {
        state.path_browse_current_dir = entry.path.clone();
    }
}

fn selected_browse_dir(state: &SetupState) -> Option<std::path::PathBuf> {
    state
        .path_browse_entries
        .get(state.path_browse_index)
        .map(|entry| std::path::PathBuf::from(shellexpand::tilde(&entry.path).as_ref()))
}

fn collapse_selected_entry(state: &mut SetupState) {
    let Some(entry) = state
        .path_browse_entries
        .get(state.path_browse_index)
        .cloned()
    else {
        return;
    };
    if !entry.expanded {
        return;
    }
    let start = state.path_browse_index + 1;
    let mut end = start;
    while end < state.path_browse_entries.len()
        && state.path_browse_entries[end].depth > entry.depth
    {
        end += 1;
    }
    if start < end {
        state.path_browse_entries.drain(start..end);
    }
    if let Some(selected) = state.path_browse_entries.get_mut(state.path_browse_index) {
        selected.expanded = false;
    }
}

fn expand_selected_entry(state: &mut SetupState) {
    let index = state.path_browse_index;
    let Some(dir) = selected_browse_dir(state) else {
        return;
    };
    let Some(selected) = state.path_browse_entries.get(index) else {
        return;
    };
    let depth = selected.depth;

    let (children, browse_error) = read_child_directories(&dir, depth + 1);
    state.path_browse_error = browse_error;
    if children.is_empty() {
        if let Some(entry) = state.path_browse_entries.get_mut(index) {
            entry.has_children = false;
            entry.expanded = false;
        }
        return;
    }

    if let Some(entry) = state.path_browse_entries.get_mut(index) {
        entry.expanded = true;
        entry.has_children = true;
    }
    state
        .path_browse_entries
        .splice(index + 1..index + 1, children);
}

fn open_selected_browse_entry(state: &mut SetupState) {
    let Some(selected) = state
        .path_browse_entries
        .get(state.path_browse_index)
        .cloned()
    else {
        return;
    };
    if !selected.has_children {
        return;
    }
    if selected.expanded {
        let child_index = state.path_browse_index + 1;
        if child_index < state.path_browse_entries.len()
            && state.path_browse_entries[child_index].depth == selected.depth + 1
        {
            state.path_browse_index = child_index;
        }
    } else {
        expand_selected_entry(state);
    }
    sync_browse_current_dir(state);
}

fn move_to_parent_or_collapse_selected_entry(state: &mut SetupState) {
    let Some(selected) = state
        .path_browse_entries
        .get(state.path_browse_index)
        .cloned()
    else {
        return;
    };
    if selected.depth == 0 {
        let root_dir = std::path::PathBuf::from(shellexpand::tilde(&selected.path).as_ref());
        if let Some(parent) = root_dir.parent() {
            let parent = parent.to_path_buf();
            set_browse_root(state, parent.clone());
            state.path_browse_info = Some(format!(
                "Moved to parent: {}",
                tilde_collapse(&parent.to_string_lossy())
            ));
        } else {
            state.path_browse_info = Some("Already at filesystem root".to_string());
        }
        return;
    }
    if selected.expanded {
        collapse_selected_entry(state);
        sync_browse_current_dir(state);
        return;
    }
    for idx in (0..state.path_browse_index).rev() {
        if state.path_browse_entries[idx].depth + 1 == selected.depth {
            state.path_browse_index = idx;
            sync_browse_current_dir(state);
            return;
        }
    }
}

fn select_current_browse_folder(state: &mut SetupState) {
    if let Some(entry) = state.path_browse_entries.get(state.path_browse_index) {
        state.base_path = entry.path.clone();
        state.path_cursor = state.base_path.len();
    }
    close_path_browse_to_input(state);
}

fn handle_path_browse(state: &mut SetupState, key: KeyEvent) {
    match key.code {
        KeyCode::Up if state.path_browse_index > 0 => {
            state.path_browse_index -= 1;
            sync_browse_current_dir(state);
        }
        KeyCode::Down if state.path_browse_index + 1 < state.path_browse_entries.len() => {
            state.path_browse_index += 1;
            sync_browse_current_dir(state);
        }
        KeyCode::Right => {
            open_selected_browse_entry(state);
        }
        KeyCode::Left => {
            move_to_parent_or_collapse_selected_entry(state);
        }
        KeyCode::Enter => {
            select_current_browse_folder(state);
        }
        KeyCode::Esc => {
            close_path_browse_to_input(state);
        }
        _ => {}
    }
}

fn handle_path_suggestions(state: &mut SetupState, key: KeyEvent) {
    match key.code {
        KeyCode::Left => {
            state.prev_step();
        }
        KeyCode::Enter => {
            confirm_path(state);
        }
        KeyCode::Char('b') => {
            open_path_browse_mode(state);
        }
        KeyCode::Esc => {
            state.prev_step();
        }
        KeyCode::Tab => open_path_browse_mode(state),
        KeyCode::Up | KeyCode::Down | KeyCode::Right => {}
        KeyCode::Backspace | KeyCode::Delete => {}
        KeyCode::Home | KeyCode::End => {}
        KeyCode::Char(_) => {}
        _ => {}
    }
}

fn handle_path_input(state: &mut SetupState, key: KeyEvent) {
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('b') {
        open_path_browse_mode(state);
        return;
    }

    match key.code {
        KeyCode::Left => {
            state.prev_step();
        }
        KeyCode::Enter => {
            confirm_path(state);
        }
        KeyCode::Char('b') => {
            open_path_browse_mode(state);
        }
        KeyCode::Esc => {
            state.prev_step();
        }
        KeyCode::Up | KeyCode::Down | KeyCode::Right => {}
        KeyCode::Tab => {}
        KeyCode::Backspace | KeyCode::Delete => {}
        KeyCode::Home | KeyCode::End => {}
        KeyCode::Char(_) => {}
        _ => {}
    }
}

async fn handle_orgs(state: &mut SetupState, key: KeyEvent) {
    if state.org_loading {
        // Discovery is triggered by a synthetic Null key in setup CLI mode.
        if key.code == KeyCode::Null {
            do_discover_orgs(state).await;
        }
        return;
    }

    match key.code {
        KeyCode::Up if state.org_index > 0 => {
            state.org_index -= 1;
        }
        KeyCode::Down if state.org_index + 1 < state.orgs.len() => {
            state.org_index += 1;
        }
        KeyCode::Char(' ') if !state.orgs.is_empty() => {
            state.orgs[state.org_index].selected = !state.orgs[state.org_index].selected;
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
                state.org_discovery_in_progress = false;
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
        state.org_discovery_in_progress = false;
        return;
    };

    let ws_provider = state.build_workspace_provider();
    match discover_org_entries(ws_provider, token.clone()).await {
        Ok(org_entries) => {
            state.orgs = org_entries;
            state.org_index = 0;
            state.org_loading = false;
            state.org_discovery_in_progress = false;
        }
        Err(e) => {
            state.org_error = Some(e);
            state.org_loading = false;
            state.org_discovery_in_progress = false;
        }
    }
}

pub(crate) async fn discover_org_entries(
    ws_provider: crate::config::WorkspaceProvider,
    token: String,
) -> Result<Vec<OrgEntry>, String> {
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
    let expanded = shellexpand::tilde(&state.base_path);
    let root = std::path::Path::new(expanded.as_ref());
    std::fs::create_dir_all(root).map_err(|e| {
        crate::errors::AppError::config(format!(
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
    Ok(())
}

#[cfg(test)]
#[path = "handler_tests.rs"]
mod tests;
