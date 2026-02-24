//! Input handler: keyboard events → state mutations (the "Update").

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tokio::sync::mpsc::UnboundedSender;

use super::app::{App, CheckEntry, Operation, OperationState, Screen};
use super::event::{AppEvent, BackendMessage};
use crate::config::WorkspaceManager;
use crate::setup::state::{SetupOutcome, SetupState, SetupStep};

/// Handle an incoming event, updating app state and optionally spawning backend work.
pub async fn handle_event(app: &mut App, event: AppEvent, backend_tx: &UnboundedSender<AppEvent>) {
    match event {
        AppEvent::Terminal(key) => handle_key(app, key, backend_tx).await,
        AppEvent::Backend(msg) => handle_backend_message(app, msg),
        AppEvent::Tick => {
            // Drive setup wizard org discovery on tick
            if app.screen == Screen::SetupWizard {
                if let Some(ref mut setup) = app.setup_state {
                    if setup.step == SetupStep::SelectOrgs && setup.org_loading {
                        crate::setup::handler::handle_key(
                            setup,
                            KeyEvent::new(KeyCode::Null, KeyModifiers::NONE),
                        )
                        .await;
                    }
                }
            }
        }
        AppEvent::Resize(_, _) => {} // ratatui handles resize
    }
}

async fn handle_key(app: &mut App, key: KeyEvent, backend_tx: &UnboundedSender<AppEvent>) {
    // Clear error message on any keypress
    app.error_message = None;

    // If filter input is active, handle text input
    if app.filter_active {
        match key.code {
            KeyCode::Esc => {
                app.filter_active = false;
                app.filter_text.clear();
            }
            KeyCode::Enter => {
                app.filter_active = false;
            }
            KeyCode::Backspace => {
                app.filter_text.pop();
            }
            KeyCode::Char(c) => {
                app.filter_text.push(c);
            }
            _ => {}
        }
        return;
    }

    // SetupWizard handles its own keys (q is valid in path input, Esc navigates steps)
    if app.screen == Screen::SetupWizard {
        // Only Ctrl+C quits the whole app from setup
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            app.should_quit = true;
            return;
        }
        handle_setup_wizard_key(app, key).await;
        return;
    }

    // Global keybindings
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        app.should_quit = true;
        return;
    }

    if key.code == KeyCode::Char('q') {
        app.should_quit = true;
        return;
    }

    if key.code == KeyCode::Esc {
        // Don't go back from InitCheck or WorkspaceSelector (they're entry points)
        if !matches!(app.screen, Screen::InitCheck | Screen::WorkspaceSelector) {
            app.go_back();
        }
        return;
    }

    // Screen-specific keybindings
    match app.screen {
        Screen::InitCheck => handle_init_check_key(app, key).await,
        Screen::SetupWizard => unreachable!(), // handled above
        Screen::WorkspaceSelector => handle_workspace_selector_key(app, key),
        Screen::Dashboard => handle_dashboard_key(app, key, backend_tx).await,
        Screen::CommandPicker => handle_picker_key(app, key, backend_tx).await,
        Screen::OrgBrowser => handle_org_browser_key(app, key),
        Screen::Progress => handle_progress_key(app, key),
        Screen::RepoStatus => handle_status_key(app, key),
    }
}

async fn handle_init_check_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Enter if !app.checks_loading => {
            // Run requirement checks
            app.checks_loading = true;
            let results = crate::checks::check_requirements().await;
            app.check_results = results
                .into_iter()
                .map(|r| CheckEntry {
                    name: r.name,
                    passed: r.passed,
                    message: r.message,
                    critical: r.critical,
                })
                .collect();
            app.checks_loading = false;
        }
        KeyCode::Char('s') => {
            // Launch setup wizard
            app.setup_state = Some(SetupState::new("~/github"));
            app.navigate_to(Screen::SetupWizard);
        }
        _ => {}
    }
}

async fn handle_setup_wizard_key(app: &mut App, key: KeyEvent) {
    let Some(ref mut setup) = app.setup_state else {
        return;
    };

    crate::setup::handler::handle_key(setup, key).await;

    if setup.should_quit {
        if matches!(setup.outcome, Some(SetupOutcome::Completed)) {
            // Reload workspaces and go to dashboard
            app.workspaces = WorkspaceManager::list().unwrap_or_default();
            if let Some(ws) = app.workspaces.first().cloned() {
                app.base_path = Some(ws.expanded_base_path());
                app.active_workspace = Some(ws);
            }
            app.setup_state = None;
            app.screen = Screen::Dashboard;
            app.screen_stack.clear();
        } else {
            // Cancelled — go to InitCheck
            app.setup_state = None;
            app.screen = Screen::InitCheck;
            app.screen_stack.clear();
        }
    }
}

fn handle_workspace_selector_key(app: &mut App, key: KeyEvent) {
    let num_ws = app.workspaces.len();
    if num_ws == 0 {
        return;
    }

    match key.code {
        KeyCode::Char('j') | KeyCode::Down => {
            app.workspace_index = (app.workspace_index + 1) % num_ws;
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.workspace_index = (app.workspace_index + num_ws - 1) % num_ws;
        }
        KeyCode::Enter => {
            app.select_workspace(app.workspace_index);
            app.screen = Screen::Dashboard;
            app.screen_stack.clear();
        }
        _ => {}
    }
}

async fn handle_dashboard_key(
    app: &mut App,
    key: KeyEvent,
    backend_tx: &UnboundedSender<AppEvent>,
) {
    match key.code {
        KeyCode::Char('s') => {
            start_operation(app, Operation::Sync, backend_tx);
        }
        KeyCode::Char('t') => {
            app.navigate_to(Screen::RepoStatus);
            start_operation(app, Operation::Status, backend_tx);
        }
        KeyCode::Char('o') => {
            app.navigate_to(Screen::OrgBrowser);
        }
        KeyCode::Char('w') => {
            if app.workspaces.len() > 1 {
                app.screen = Screen::WorkspaceSelector;
                app.screen_stack.clear();
            }
        }
        KeyCode::Enter => {
            app.navigate_to(Screen::CommandPicker);
        }
        _ => {}
    }
}

async fn handle_picker_key(app: &mut App, key: KeyEvent, backend_tx: &UnboundedSender<AppEvent>) {
    let num_items = 2; // Sync, Status
    match key.code {
        KeyCode::Char('j') | KeyCode::Down => {
            app.picker_index = (app.picker_index + 1) % num_items;
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.picker_index = (app.picker_index + num_items - 1) % num_items;
        }
        KeyCode::Char('d') => {
            app.dry_run = !app.dry_run;
        }
        KeyCode::Char('m') => {
            app.sync_pull = !app.sync_pull;
        }
        KeyCode::Enter => {
            let operation = match app.picker_index {
                0 => Operation::Sync,
                1 => Operation::Status,
                _ => return,
            };
            start_operation(app, operation, backend_tx);
        }
        _ => {}
    }
}

fn handle_org_browser_key(app: &mut App, key: KeyEvent) {
    match key.code {
        // Shift+J/K for org navigation
        KeyCode::Char('J') => {
            if !app.orgs.is_empty() {
                app.org_index = (app.org_index + 1) % app.orgs.len();
                app.repo_index = 0;
            }
        }
        KeyCode::Char('K') => {
            if !app.orgs.is_empty() {
                app.org_index = (app.org_index + app.orgs.len() - 1) % app.orgs.len();
                app.repo_index = 0;
            }
        }
        // j/k for repo navigation within selected org
        KeyCode::Char('j') | KeyCode::Down => {
            let repo_count = current_org_repo_count(app);
            if repo_count > 0 {
                app.repo_index = (app.repo_index + 1) % repo_count;
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            let repo_count = current_org_repo_count(app);
            if repo_count > 0 {
                app.repo_index = (app.repo_index + repo_count - 1) % repo_count;
            }
        }
        KeyCode::Char('/') => {
            app.filter_active = true;
            app.filter_text.clear();
        }
        _ => {}
    }
}

fn handle_progress_key(app: &mut App, key: KeyEvent) {
    match key.code {
        // Scroll log
        KeyCode::Char('j') | KeyCode::Down => {
            if app.scroll_offset < app.log_lines.len().saturating_sub(1) {
                app.scroll_offset += 1;
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.scroll_offset = app.scroll_offset.saturating_sub(1);
        }
        _ => {}
    }
}

fn handle_status_key(app: &mut App, key: KeyEvent) {
    let filtered_count = filtered_repo_count(app);
    match key.code {
        KeyCode::Char('j') | KeyCode::Down => {
            if filtered_count > 0 {
                app.repo_index = (app.repo_index + 1) % filtered_count;
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if filtered_count > 0 {
                app.repo_index = (app.repo_index + filtered_count - 1) % filtered_count;
            }
        }
        KeyCode::Char('D') => {
            app.filter_dirty = !app.filter_dirty;
            app.repo_index = 0;
        }
        KeyCode::Char('B') => {
            app.filter_behind = !app.filter_behind;
            app.repo_index = 0;
        }
        KeyCode::Char('/') => {
            app.filter_active = true;
            app.filter_text.clear();
        }
        _ => {}
    }
}

fn start_operation(app: &mut App, operation: Operation, backend_tx: &UnboundedSender<AppEvent>) {
    if matches!(app.operation_state, OperationState::Running { .. }) {
        app.error_message = Some("An operation is already running".to_string());
        return;
    }

    app.operation_state = OperationState::Discovering {
        message: format!("Starting {}...", operation),
    };
    app.log_lines.clear();
    app.scroll_offset = 0;

    if !matches!(app.screen, Screen::Progress | Screen::RepoStatus) {
        app.navigate_to(Screen::Progress);
    }

    super::backend::spawn_operation(operation, app, backend_tx.clone());
}

fn current_org_repo_count(app: &App) -> usize {
    app.orgs
        .get(app.org_index)
        .and_then(|org| app.repos_by_org.get(org))
        .map(|repos| repos.len())
        .unwrap_or(0)
}

fn filtered_repo_count(app: &App) -> usize {
    app.local_repos
        .iter()
        .filter(|r| {
            if app.filter_dirty && !r.is_dirty {
                return false;
            }
            if app.filter_behind && r.behind == 0 {
                return false;
            }
            if !app.filter_text.is_empty()
                && !r
                    .full_name
                    .to_lowercase()
                    .contains(&app.filter_text.to_lowercase())
            {
                return false;
            }
            true
        })
        .count()
}

fn handle_backend_message(app: &mut App, msg: BackendMessage) {
    match msg {
        BackendMessage::OrgsDiscovered(count) => {
            app.operation_state = OperationState::Discovering {
                message: format!("Found {} organizations", count),
            };
        }
        BackendMessage::OrgStarted(name) => {
            app.operation_state = OperationState::Discovering {
                message: format!("Discovering: {}", name),
            };
        }
        BackendMessage::OrgComplete(name, count) => {
            app.log_lines
                .push(format!("[ok] {} ({} repos)", name, count));
        }
        BackendMessage::DiscoveryComplete(repos) => {
            // Populate org data
            let mut by_org: std::collections::HashMap<String, Vec<_>> =
                std::collections::HashMap::new();
            for repo in &repos {
                by_org
                    .entry(repo.owner.clone())
                    .or_default()
                    .push(repo.clone());
            }
            let mut org_names: Vec<String> = by_org.keys().cloned().collect();
            org_names.sort();
            app.orgs = org_names;
            app.repos_by_org = by_org;
            app.all_repos = repos;
        }
        BackendMessage::DiscoveryError(msg) => {
            app.operation_state = OperationState::Idle;
            app.error_message = Some(msg);
        }
        BackendMessage::RepoProgress {
            repo_name,
            success,
            message,
        } => {
            if let OperationState::Running {
                ref mut completed,
                ref mut failed,
                ref mut current_repo,
                ..
            } = app.operation_state
            {
                *completed += 1;
                *current_repo = repo_name.clone();
                if !success {
                    *failed += 1;
                }
            }
            let prefix = if success { "[ok]" } else { "[!!]" };
            app.log_lines
                .push(format!("{} {} - {}", prefix, repo_name, message));
            // Auto-scroll to bottom
            app.scroll_offset = app.log_lines.len().saturating_sub(1);
        }
        BackendMessage::OperationComplete(summary) => {
            let op = match &app.operation_state {
                OperationState::Running { operation, .. } => *operation,
                _ => Operation::Sync,
            };
            app.operation_state = OperationState::Finished {
                operation: op,
                summary,
            };
        }
        BackendMessage::OperationError(msg) => {
            app.operation_state = OperationState::Idle;
            app.error_message = Some(msg);
        }
        BackendMessage::StatusResults(entries) => {
            app.local_repos = entries;
            app.operation_state = OperationState::Idle;
        }
    }
}
