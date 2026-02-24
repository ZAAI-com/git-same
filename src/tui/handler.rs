//! Input handler: keyboard events → state mutations (the "Update").

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tokio::sync::mpsc::UnboundedSender;

use super::app::{App, CheckEntry, Operation, OperationState, Screen};
use super::event::{AppEvent, BackendMessage};
use crate::config::{Config, WorkspaceManager};
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
            // Run background requirement checks when on Dashboard
            if app.screen == Screen::Dashboard
                && app.check_results.is_empty()
                && !app.checks_loading
            {
                app.checks_loading = true;
                let tx = backend_tx.clone();
                tokio::spawn(async move {
                    let results = crate::checks::check_requirements().await;
                    let entries: Vec<CheckEntry> = results
                        .into_iter()
                        .map(|r| CheckEntry {
                            name: r.name,
                            passed: r.passed,
                            message: r.message,
                            critical: r.critical,
                        })
                        .collect();
                    let _ = tx.send(AppEvent::Backend(BackendMessage::CheckResults(entries)));
                });
            }
            // Auto-trigger status scan to populate dashboard stats
            if app.screen == Screen::Dashboard
                && app.active_workspace.is_some()
                && app.local_repos.is_empty()
                && !app.status_loading
            {
                app.status_loading = true;
                super::backend::spawn_operation(
                    Operation::Status,
                    app,
                    backend_tx.clone(),
                );
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
        // Don't go back from entry points (no screen stack)
        if app.screen == Screen::InitCheck {
            return;
        }
        // WorkspaceSelector: only go back if navigated to (has screen stack)
        if app.screen == Screen::WorkspaceSelector && app.screen_stack.is_empty() {
            return;
        }
        app.go_back();
        return;
    }

    // Screen-specific keybindings
    match app.screen {
        Screen::InitCheck => handle_init_check_key(app, key, backend_tx).await,
        Screen::SetupWizard => unreachable!(), // handled above
        Screen::WorkspaceSelector => {
            handle_workspace_selector_key(app, key, backend_tx).await;
        }
        Screen::Dashboard => handle_dashboard_key(app, key, backend_tx).await,
        Screen::CommandPicker => handle_picker_key(app, key, backend_tx).await,
        Screen::OrgBrowser => handle_org_browser_key(app, key),
        Screen::Progress => handle_progress_key(app, key),
        Screen::RepoStatus => handle_status_key(app, key, backend_tx),
        Screen::Settings => handle_settings_key(app, key),
    }
}

async fn handle_init_check_key(
    app: &mut App,
    key: KeyEvent,
    backend_tx: &UnboundedSender<AppEvent>,
) {
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
        KeyCode::Char('c') if !app.check_results.is_empty() && !app.config_created => {
            // Create config file
            let tx = backend_tx.clone();
            tokio::spawn(async move {
                match Config::default_path() {
                    Ok(config_path) => {
                        if config_path.exists() {
                            let _ = tx.send(AppEvent::Backend(BackendMessage::InitConfigError(
                                format!(
                                    "Config already exists at {}. Delete it first to recreate.",
                                    config_path.display()
                                ),
                            )));
                            return;
                        }
                        if let Some(parent) = config_path.parent() {
                            if let Err(e) = std::fs::create_dir_all(parent) {
                                let _ =
                                    tx.send(AppEvent::Backend(BackendMessage::InitConfigError(
                                        format!("Failed to create config directory: {}", e),
                                    )));
                                return;
                            }
                        }
                        let default_config = Config::default_toml();
                        match std::fs::write(&config_path, default_config) {
                            Ok(()) => {
                                let _ =
                                    tx.send(AppEvent::Backend(BackendMessage::InitConfigCreated(
                                        config_path.display().to_string(),
                                    )));
                            }
                            Err(e) => {
                                let _ =
                                    tx.send(AppEvent::Backend(BackendMessage::InitConfigError(
                                        format!("Failed to write config: {}", e),
                                    )));
                            }
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(AppEvent::Backend(BackendMessage::InitConfigError(
                            format!("Cannot determine config path: {}", e),
                        )));
                    }
                }
            });
        }
        KeyCode::Char('s') => {
            // Launch setup wizard
            let default_path = std::env::current_dir()
                .map(|p| crate::setup::state::tilde_collapse(&p.to_string_lossy()))
                .unwrap_or_else(|_| "~/Git-Same/GitHub".to_string());
            app.setup_state = Some(SetupState::new(&default_path));
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

async fn handle_workspace_selector_key(
    app: &mut App,
    key: KeyEvent,
    backend_tx: &UnboundedSender<AppEvent>,
) {
    let num_ws = app.workspaces.len();

    match key.code {
        KeyCode::Char('n') => {
            // Launch setup wizard to create a new workspace
            let default_path = std::env::current_dir()
                .map(|p| crate::setup::state::tilde_collapse(&p.to_string_lossy()))
                .unwrap_or_else(|_| "~/Git-Same/GitHub".to_string());
            app.setup_state = Some(SetupState::new(&default_path));
            app.navigate_to(Screen::SetupWizard);
        }
        KeyCode::Char('j') | KeyCode::Down if num_ws > 0 => {
            app.workspace_index = (app.workspace_index + 1) % num_ws;
        }
        KeyCode::Char('k') | KeyCode::Up if num_ws > 0 => {
            app.workspace_index = (app.workspace_index + num_ws - 1) % num_ws;
        }
        KeyCode::Char('d') if num_ws > 0 => {
            // Toggle default workspace
            if let Some(ws) = app.workspaces.get(app.workspace_index) {
                let ws_name = ws.name.clone();
                let is_already_default = app.config.default_workspace.as_deref() == Some(&ws_name);
                let new_default = if is_already_default {
                    None
                } else {
                    Some(ws_name)
                };
                let tx = backend_tx.clone();
                let default_clone = new_default.clone();
                tokio::spawn(async move {
                    match Config::save_default_workspace(default_clone.as_deref()) {
                        Ok(()) => {
                            let _ = tx.send(AppEvent::Backend(
                                BackendMessage::DefaultWorkspaceUpdated(default_clone),
                            ));
                        }
                        Err(e) => {
                            let _ = tx.send(AppEvent::Backend(
                                BackendMessage::DefaultWorkspaceError(format!("{}", e)),
                            ));
                        }
                    }
                });
            }
        }
        KeyCode::Enter if num_ws > 0 => {
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
            app.status_loading = true;
            app.navigate_to(Screen::RepoStatus);
            start_operation(app, Operation::Status, backend_tx);
        }
        KeyCode::Char('o') => {
            app.navigate_to(Screen::OrgBrowser);
        }
        KeyCode::Char('e') => {
            app.navigate_to(Screen::Settings);
        }
        KeyCode::Char('w') => {
            app.navigate_to(Screen::WorkspaceSelector);
        }
        KeyCode::Char('m') | KeyCode::Enter => {
            app.navigate_to(Screen::CommandPicker);
        }
        KeyCode::Left | KeyCode::Char('h') => {
            app.stat_index = app.stat_index.saturating_sub(1);
        }
        KeyCode::Right | KeyCode::Char('l') => {
            if app.stat_index < 5 {
                app.stat_index += 1;
            }
        }
        _ => {}
    }
}

fn handle_settings_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('c') => {
            // Open config directory in Finder / file manager
            if let Ok(path) = crate::config::Config::default_path() {
                if let Some(parent) = path.parent() {
                    let _ = std::process::Command::new("open").arg(parent).spawn();
                }
            }
        }
        KeyCode::Char('d') => {
            app.dry_run = !app.dry_run;
        }
        KeyCode::Char('m') => {
            app.sync_pull = !app.sync_pull;
        }
        KeyCode::Char(c @ '1'..='9') => {
            // Open workspace folder by number key
            let index = (c as usize) - ('1' as usize);
            if let Some(ws) = app.workspaces.get(index) {
                let path = ws.expanded_base_path();
                let _ = std::process::Command::new("open").arg(&path).spawn();
            }
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

fn handle_status_key(app: &mut App, key: KeyEvent, backend_tx: &UnboundedSender<AppEvent>) {
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
        KeyCode::Char('r') => {
            app.status_loading = true;
            start_operation(app, Operation::Status, backend_tx);
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
        BackendMessage::OperationStarted { operation, total } => {
            app.log_lines.clear();
            app.operation_state = OperationState::Running {
                operation,
                total,
                completed: 0,
                failed: 0,
                skipped: 0,
                current_repo: String::new(),
            };
        }
        BackendMessage::RepoProgress {
            repo_name,
            success,
            skipped,
            message,
        } => {
            if let OperationState::Running {
                ref mut completed,
                ref mut failed,
                skipped: ref mut skip_count,
                ref mut current_repo,
                ..
            } = app.operation_state
            {
                *completed += 1;
                *current_repo = repo_name.clone();
                if skipped {
                    *skip_count += 1;
                } else if !success {
                    *failed += 1;
                }
            }
            let prefix = if !success {
                "[!!]"
            } else if skipped {
                "[--]"
            } else {
                "[ok]"
            };
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
            // Update last_synced after a successful sync
            if op == Operation::Sync {
                let now = chrono::Utc::now().to_rfc3339();
                if let Some(ref mut ws) = app.active_workspace {
                    ws.last_synced = Some(now.clone());
                    let _ = WorkspaceManager::save(ws);
                    // Keep workspaces list in sync
                    if let Some(entry) = app.workspaces.iter_mut().find(|w| w.name == ws.name) {
                        entry.last_synced = Some(now);
                    }
                }
            }
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
            app.status_loading = false;
        }
        BackendMessage::InitConfigCreated(path) => {
            app.config_created = true;
            app.config_path_display = Some(path);
        }
        BackendMessage::InitConfigError(msg) => {
            app.error_message = Some(msg);
        }
        BackendMessage::DefaultWorkspaceUpdated(name) => {
            app.config.default_workspace = name;
        }
        BackendMessage::DefaultWorkspaceError(msg) => {
            app.error_message = Some(msg);
        }
        BackendMessage::CheckResults(entries) => {
            app.check_results = entries;
            app.checks_loading = false;
        }
    }
}
