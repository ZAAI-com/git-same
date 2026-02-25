//! Input handler: keyboard events → state mutations (the "Update").

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tokio::sync::mpsc::UnboundedSender;

use super::app::{
    App, CheckEntry, LogFilter, Operation, OperationState, Screen, SyncHistoryEntry, SyncLogEntry,
    SyncLogStatus,
};
use super::event::{AppEvent, BackendMessage};
use crate::cache::SyncHistoryManager;
use crate::config::{Config, WorkspaceManager};
use crate::setup::state::{SetupOutcome, SetupState, SetupStep};

/// Handle an incoming event, updating app state and optionally spawning backend work.
pub async fn handle_event(app: &mut App, event: AppEvent, backend_tx: &UnboundedSender<AppEvent>) {
    match event {
        AppEvent::Terminal(key) => handle_key(app, key, backend_tx).await,
        AppEvent::Backend(msg) => handle_backend_message(app, msg, backend_tx),
        AppEvent::Tick => {
            // Increment animation tick counter on Progress screen during active ops
            if app.screen == Screen::Progress
                && matches!(
                    &app.operation_state,
                    OperationState::Discovering { .. } | OperationState::Running { .. }
                )
            {
                app.tick_count = app.tick_count.wrapping_add(1);

                // Sample throughput every 10 ticks (1 second at 100ms tick rate)
                if app.tick_count.is_multiple_of(10) {
                    if let OperationState::Running {
                        completed,
                        ref mut throughput_samples,
                        ref mut last_sample_completed,
                        ..
                    } = app.operation_state
                    {
                        let delta = completed.saturating_sub(*last_sample_completed) as u64;
                        throughput_samples.push(delta);
                        *last_sample_completed = completed;
                    }
                }
            }
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
            // Auto-trigger status scan when data is stale or missing
            let refresh_interval = app
                .active_workspace
                .as_ref()
                .and_then(|ws| ws.refresh_interval)
                .unwrap_or(app.config.refresh_interval);
            if app.screen == Screen::Dashboard
                && app.active_workspace.is_some()
                && !app.status_loading
                && app
                    .last_status_scan
                    .is_none_or(|t| t.elapsed().as_secs() >= refresh_interval)
            {
                app.status_loading = true;
                super::backend::spawn_operation(Operation::Status, app, backend_tx.clone());
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
        if app.quit_pending {
            app.should_quit = true;
        } else {
            app.quit_pending = true;
        }
        return;
    }
    app.quit_pending = false;

    if key.code == KeyCode::Esc {
        // Don't go back from entry points (no screen stack)
        if app.screen == Screen::InitCheck {
            return;
        }
        // Workspace screen: only go back if navigated to (has screen stack)
        if app.screen == Screen::Workspace && app.screen_stack.is_empty() {
            return;
        }
        app.go_back();
        return;
    }

    // Screen-specific keybindings
    match app.screen {
        Screen::InitCheck => handle_init_check_key(app, key, backend_tx).await,
        Screen::SetupWizard => unreachable!(), // handled above
        Screen::Workspace => {
            handle_workspace_key(app, key, backend_tx).await;
        }
        Screen::Dashboard => handle_dashboard_key(app, key, backend_tx).await,
        Screen::Progress => handle_progress_key(app, key, backend_tx),
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
                app.sync_history = SyncHistoryManager::for_workspace(&ws.name)
                    .and_then(|m| m.load())
                    .unwrap_or_default();
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

async fn handle_workspace_key(
    app: &mut App,
    key: KeyEvent,
    backend_tx: &UnboundedSender<AppEvent>,
) {
    let num_ws = app.workspaces.len();
    let total_entries = num_ws + 1; // workspaces + "Create Workspace"

    match key.code {
        KeyCode::Char('j') | KeyCode::Down | KeyCode::Tab if total_entries > 0 => {
            app.workspace_index = (app.workspace_index + 1) % total_entries;
            app.settings_config_expanded = false;
        }
        KeyCode::Char('k') | KeyCode::Up if total_entries > 0 => {
            app.workspace_index = (app.workspace_index + total_entries - 1) % total_entries;
            app.settings_config_expanded = false;
        }
        KeyCode::Enter => {
            if app.workspace_index < num_ws {
                // On a workspace entry
                let is_active = app
                    .active_workspace
                    .as_ref()
                    .map(|aw| aw.name == app.workspaces[app.workspace_index].name)
                    .unwrap_or(false);
                if is_active {
                    // Toggle config expansion
                    app.settings_config_expanded = !app.settings_config_expanded;
                } else {
                    // Switch active workspace and go to dashboard
                    app.select_workspace(app.workspace_index);
                    app.screen = Screen::Dashboard;
                    app.screen_stack.clear();
                }
            } else {
                // "Create Workspace" entry
                let default_path = std::env::current_dir()
                    .map(|p| crate::setup::state::tilde_collapse(&p.to_string_lossy()))
                    .unwrap_or_else(|_| "~/Git-Same/GitHub".to_string());
                app.setup_state = Some(SetupState::new(&default_path));
                app.navigate_to(Screen::SetupWizard);
            }
        }
        KeyCode::Char('n') => {
            // Shortcut to create workspace
            let default_path = std::env::current_dir()
                .map(|p| crate::setup::state::tilde_collapse(&p.to_string_lossy()))
                .unwrap_or_else(|_| "~/Git-Same/GitHub".to_string());
            app.setup_state = Some(SetupState::new(&default_path));
            app.navigate_to(Screen::SetupWizard);
        }
        KeyCode::Char('d') if app.workspace_index < num_ws => {
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
        KeyCode::Char('o') if app.workspace_index < num_ws => {
            // Open workspace folder
            if let Some(ws) = app.workspaces.get(app.workspace_index) {
                let path = ws.expanded_base_path();
                let _ = std::process::Command::new("open").arg(&path).spawn();
            }
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
            app.last_status_scan = None; // Force immediate refresh
            app.status_loading = true;
            start_operation(app, Operation::Status, backend_tx);
        }
        // Tab shortcuts
        KeyCode::Char('o') => {
            app.stat_index = 0;
            app.dashboard_table_state.select(Some(0));
        }
        KeyCode::Char('r') => {
            app.stat_index = 1;
            app.dashboard_table_state.select(Some(0));
        }
        KeyCode::Char('c') => {
            app.stat_index = 2;
            app.dashboard_table_state.select(Some(0));
        }
        KeyCode::Char('b') => {
            app.stat_index = 3;
            app.dashboard_table_state.select(Some(0));
        }
        KeyCode::Char('a') => {
            app.stat_index = 4;
            app.dashboard_table_state.select(Some(0));
        }
        KeyCode::Char('u') => {
            app.stat_index = 5;
            app.dashboard_table_state.select(Some(0));
        }
        KeyCode::Char('e') => {
            app.navigate_to(Screen::Settings);
        }
        KeyCode::Char('w') => {
            app.navigate_to(Screen::Workspace);
        }
        KeyCode::Char('i') => {
            app.navigate_to(Screen::InitCheck);
        }
        KeyCode::Char('/') => {
            app.filter_active = true;
            app.filter_text.clear();
            app.stat_index = 1;
            app.dashboard_table_state.select(Some(0));
        }
        // Tab navigation (left/right between stat boxes)
        KeyCode::Left | KeyCode::Char('h') => {
            app.stat_index = app.stat_index.saturating_sub(1);
            app.dashboard_table_state.select(Some(0));
        }
        KeyCode::Right | KeyCode::Char('l') => {
            if app.stat_index < 5 {
                app.stat_index += 1;
                app.dashboard_table_state.select(Some(0));
            }
        }
        // List navigation (up/down within tab content)
        KeyCode::Down | KeyCode::Char('j') => {
            let count = dashboard_tab_item_count(app);
            if count > 0 {
                let current = app.dashboard_table_state.selected().unwrap_or(0);
                if current + 1 < count {
                    app.dashboard_table_state.select(Some(current + 1));
                }
            }
        }
        KeyCode::Up | KeyCode::Char('k') => {
            let count = dashboard_tab_item_count(app);
            if count > 0 {
                let current = app.dashboard_table_state.selected().unwrap_or(0);
                app.dashboard_table_state
                    .select(Some(current.saturating_sub(1)));
            }
        }
        KeyCode::Enter => {
            // Open the selected repo's folder
            if let Some(path) = dashboard_selected_repo_path(app) {
                let _ = std::process::Command::new("open").arg(&path).spawn();
            }
        }
        _ => {}
    }
}

fn handle_settings_key(app: &mut App, key: KeyEvent) {
    let num_items = 2; // Requirements, Options
    match key.code {
        KeyCode::Tab | KeyCode::Down => {
            app.settings_index = (app.settings_index + 1) % num_items;
        }
        KeyCode::Up => {
            app.settings_index = (app.settings_index + num_items - 1) % num_items;
        }
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
        _ => {}
    }
}

fn handle_progress_key(app: &mut App, key: KeyEvent, backend_tx: &UnboundedSender<AppEvent>) {
    let is_finished = matches!(app.operation_state, OperationState::Finished { .. });

    match key.code {
        // Scroll log
        KeyCode::Char('j') | KeyCode::Down => {
            if is_finished {
                let count = filtered_log_count(app);
                if count > 0 && app.sync_log_index < count.saturating_sub(1) {
                    app.sync_log_index += 1;
                }
            } else if app.scroll_offset < app.log_lines.len().saturating_sub(1) {
                app.scroll_offset += 1;
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if is_finished {
                app.sync_log_index = app.sync_log_index.saturating_sub(1);
            } else {
                app.scroll_offset = app.scroll_offset.saturating_sub(1);
            }
        }
        // Expand/collapse commit deep dive
        KeyCode::Enter if is_finished => {
            // Extract data we need before mutating app
            let selected = filtered_log_entries(app)
                .get(app.sync_log_index)
                .map(|e| (e.repo_name.clone(), e.path.clone()));

            if let Some((repo_name, path)) = selected {
                if app.expanded_repo.as_deref() == Some(&repo_name) {
                    // Toggle off: collapse
                    app.expanded_repo = None;
                    app.repo_commits.clear();
                } else if let Some(path) = path {
                    // Expand: fetch commits
                    app.expanded_repo = Some(repo_name.clone());
                    app.repo_commits.clear();
                    super::backend::spawn_commit_fetch(path, repo_name, backend_tx.clone());
                }
            }
        }
        // Post-sync log filters
        KeyCode::Char('a') if is_finished => {
            app.log_filter = LogFilter::All;
            app.sync_log_index = 0;
            app.expanded_repo = None;
            app.repo_commits.clear();
        }
        KeyCode::Char('u') if is_finished => {
            app.log_filter = LogFilter::Updated;
            app.sync_log_index = 0;
            app.expanded_repo = None;
            app.repo_commits.clear();
        }
        KeyCode::Char('f') if is_finished => {
            app.log_filter = LogFilter::Failed;
            app.sync_log_index = 0;
            app.expanded_repo = None;
            app.repo_commits.clear();
        }
        KeyCode::Char('x') if is_finished => {
            app.log_filter = LogFilter::Skipped;
            app.sync_log_index = 0;
            app.expanded_repo = None;
            app.repo_commits.clear();
        }
        KeyCode::Char('c') if is_finished => {
            app.log_filter = LogFilter::Changelog;
            app.sync_log_index = 0;
            app.expanded_repo = None;
            app.repo_commits.clear();
        }
        // Sync history overlay toggle
        KeyCode::Char('h') if is_finished => {
            app.show_sync_history = !app.show_sync_history;
        }
        _ => {}
    }
}

/// Count of log entries matching the current filter.
fn filtered_log_count(app: &App) -> usize {
    match app.log_filter {
        LogFilter::All => app.sync_log_entries.len(),
        LogFilter::Updated => app
            .sync_log_entries
            .iter()
            .filter(|e| e.had_updates || e.is_clone)
            .count(),
        LogFilter::Failed => app
            .sync_log_entries
            .iter()
            .filter(|e| e.status == SyncLogStatus::Failed)
            .count(),
        LogFilter::Skipped => app
            .sync_log_entries
            .iter()
            .filter(|e| e.status == SyncLogStatus::Skipped)
            .count(),
        LogFilter::Changelog => app
            .sync_log_entries
            .iter()
            .filter(|e| e.had_updates)
            .count(),
    }
}

/// Returns filtered log entries matching the current filter.
fn filtered_log_entries(app: &App) -> Vec<&SyncLogEntry> {
    match app.log_filter {
        LogFilter::All => app.sync_log_entries.iter().collect(),
        LogFilter::Updated => app
            .sync_log_entries
            .iter()
            .filter(|e| e.had_updates || e.is_clone)
            .collect(),
        LogFilter::Failed => app
            .sync_log_entries
            .iter()
            .filter(|e| e.status == SyncLogStatus::Failed)
            .collect(),
        LogFilter::Skipped => app
            .sync_log_entries
            .iter()
            .filter(|e| e.status == SyncLogStatus::Skipped)
            .collect(),
        LogFilter::Changelog => app
            .sync_log_entries
            .iter()
            .filter(|e| e.had_updates)
            .collect(),
    }
}

/// Compute the filesystem path for a repo from its full name (e.g. "org/repo").
/// Mirrors `DiscoveryOrchestrator::compute_path()` logic using workspace config.
fn compute_repo_path(app: &App, repo_name: &str) -> Option<std::path::PathBuf> {
    let ws = app.active_workspace.as_ref()?;
    let base_path = ws.expanded_base_path();
    let structure = ws
        .structure
        .clone()
        .unwrap_or_else(|| app.config.structure.clone());
    let parts: Vec<&str> = repo_name.splitn(2, '/').collect();
    if parts.len() != 2 {
        return None;
    }
    let (org, repo) = (parts[0], parts[1]);
    let provider_name = ws.provider.kind.to_string().to_lowercase();
    let path_str = structure
        .replace("{provider}", &provider_name)
        .replace("{org}", org)
        .replace("{repo}", repo);
    Some(base_path.join(path_str))
}

fn start_operation(app: &mut App, operation: Operation, backend_tx: &UnboundedSender<AppEvent>) {
    if matches!(app.operation_state, OperationState::Running { .. }) {
        app.error_message = Some("An operation is already running".to_string());
        return;
    }

    app.tick_count = 0;
    app.operation_state = OperationState::Discovering {
        message: format!("Starting {}...", operation),
    };
    app.log_lines.clear();
    app.scroll_offset = 0;

    if !matches!(app.screen, Screen::Progress) {
        app.navigate_to(Screen::Progress);
    }

    super::backend::spawn_operation(operation, app, backend_tx.clone());
}

fn dashboard_tab_item_count(app: &App) -> usize {
    match app.stat_index {
        0 => app
            .local_repos
            .iter()
            .map(|r| r.owner.as_str())
            .collect::<std::collections::HashSet<_>>()
            .len(),
        1 => {
            if app.filter_text.is_empty() {
                app.local_repos.len()
            } else {
                let ft = app.filter_text.to_lowercase();
                app.local_repos
                    .iter()
                    .filter(|r| r.full_name.to_lowercase().contains(&ft))
                    .count()
            }
        }
        2 => app
            .local_repos
            .iter()
            .filter(|r| !r.is_uncommitted && r.behind == 0 && r.ahead == 0)
            .count(),
        3 => app.local_repos.iter().filter(|r| r.behind > 0).count(),
        4 => app.local_repos.iter().filter(|r| r.ahead > 0).count(),
        5 => app.local_repos.iter().filter(|r| r.is_uncommitted).count(),
        _ => 0,
    }
}

fn dashboard_selected_repo_path(app: &App) -> Option<std::path::PathBuf> {
    let selected = app.dashboard_table_state.selected()?;
    let repos: Vec<&super::app::RepoEntry> = match app.stat_index {
        0 => return None, // Owners tab — no single repo
        1 => {
            if app.filter_text.is_empty() {
                app.local_repos.iter().collect()
            } else {
                let ft = app.filter_text.to_lowercase();
                app.local_repos
                    .iter()
                    .filter(|r| r.full_name.to_lowercase().contains(&ft))
                    .collect()
            }
        }
        2 => app
            .local_repos
            .iter()
            .filter(|r| !r.is_uncommitted && r.behind == 0 && r.ahead == 0)
            .collect(),
        3 => app.local_repos.iter().filter(|r| r.behind > 0).collect(),
        4 => app.local_repos.iter().filter(|r| r.ahead > 0).collect(),
        5 => app
            .local_repos
            .iter()
            .filter(|r| r.is_uncommitted)
            .collect(),
        _ => return None,
    };
    repos.get(selected).map(|r| r.path.clone())
}

fn handle_backend_message(
    app: &mut App,
    msg: BackendMessage,
    backend_tx: &UnboundedSender<AppEvent>,
) {
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
        BackendMessage::OperationStarted {
            operation,
            total,
            to_clone,
            to_sync,
        } => {
            app.log_lines.clear();
            app.sync_log_entries.clear();
            app.log_filter = LogFilter::All;
            app.sync_log_index = 0;
            app.expanded_repo = None;
            app.repo_commits.clear();
            app.show_sync_history = false;
            app.operation_state = OperationState::Running {
                operation,
                total,
                completed: 0,
                failed: 0,
                skipped: 0,
                current_repo: String::new(),
                with_updates: 0,
                cloned: 0,
                synced: 0,
                to_clone,
                to_sync,
                total_new_commits: 0,
                started_at: std::time::Instant::now(),
                active_repos: Vec::new(),
                throughput_samples: Vec::new(),
                last_sample_completed: 0,
            };
        }
        BackendMessage::RepoStarted { repo_name } => {
            if let OperationState::Running {
                ref mut active_repos,
                ..
            } = app.operation_state
            {
                active_repos.push(repo_name);
            }
        }
        BackendMessage::RepoProgress {
            repo_name,
            success,
            skipped,
            message,
            had_updates,
            is_clone,
            new_commits,
            skip_reason: _,
        } => {
            if let OperationState::Running {
                ref mut completed,
                ref mut failed,
                skipped: ref mut skip_count,
                ref mut current_repo,
                ref mut with_updates,
                ref mut cloned,
                ref mut synced,
                ref mut total_new_commits,
                ref mut active_repos,
                ..
            } = app.operation_state
            {
                *completed += 1;
                *current_repo = repo_name.clone();

                // Remove from active workers
                active_repos.retain(|r| r != &repo_name);

                if skipped {
                    *skip_count += 1;
                } else if !success {
                    *failed += 1;
                } else {
                    if is_clone {
                        *cloned += 1;
                    } else {
                        *synced += 1;
                    }
                    if had_updates {
                        *with_updates += 1;
                        if let Some(n) = new_commits {
                            *total_new_commits += n;
                        }
                    }
                }
            }

            // Build structured log entry
            let log_status = if !success {
                SyncLogStatus::Failed
            } else if skipped {
                SyncLogStatus::Skipped
            } else if is_clone {
                SyncLogStatus::Cloned
            } else if had_updates {
                SyncLogStatus::Updated
            } else {
                SyncLogStatus::Success
            };

            app.sync_log_entries.push(SyncLogEntry {
                repo_name: repo_name.clone(),
                status: log_status,
                message: message.clone(),
                had_updates,
                is_clone,
                new_commits,
                path: compute_repo_path(app, &repo_name),
            });

            // Build legacy log line with enriched prefixes
            let prefix = match log_status {
                SyncLogStatus::Failed => "[!!]",
                SyncLogStatus::Skipped => "[--]",
                SyncLogStatus::Cloned => "[++]",
                SyncLogStatus::Updated => "[**]",
                SyncLogStatus::Success => "[ok]",
            };

            let commit_info = if had_updates {
                if let Some(n) = new_commits {
                    if n > 0 {
                        format!(" ({} new commits)", n)
                    } else {
                        String::new()
                    }
                } else {
                    String::new()
                }
            } else {
                String::new()
            };

            app.log_lines.push(format!(
                "{} {} - {}{}",
                prefix, repo_name, message, commit_info
            ));
            // Auto-scroll to bottom
            app.scroll_offset = app.log_lines.len().saturating_sub(1);
        }
        BackendMessage::OperationComplete(summary) => {
            // Extract accumulated metrics from Running state before transitioning
            let (op, wu, cl, sy, tnc, dur) = match &app.operation_state {
                OperationState::Running {
                    operation,
                    with_updates,
                    cloned,
                    synced,
                    total_new_commits,
                    started_at,
                    ..
                } => (
                    *operation,
                    *with_updates,
                    *cloned,
                    *synced,
                    *total_new_commits,
                    started_at.elapsed().as_secs_f64(),
                ),
                _ => (Operation::Sync, 0, 0, 0, 0, 0.0),
            };

            // Update last_synced after a successful sync
            if op == Operation::Sync {
                let now = chrono::Utc::now().to_rfc3339();
                if let Some(ref mut ws) = app.active_workspace {
                    ws.last_synced = Some(now.clone());
                    let _ = WorkspaceManager::save(ws);
                    if let Some(entry) = app.workspaces.iter_mut().find(|w| w.name == ws.name) {
                        entry.last_synced = Some(now.clone());
                    }
                }

                // Save to sync history
                app.sync_history.push(SyncHistoryEntry {
                    timestamp: now,
                    duration_secs: dur,
                    success: summary.success,
                    failed: summary.failed,
                    skipped: summary.skipped,
                    with_updates: wu,
                    cloned: cl,
                    total_new_commits: tnc,
                });
                // Cap in-memory history
                if app.sync_history.len() > 50 {
                    app.sync_history.remove(0);
                }

                // Persist history to disk
                if let Some(ref ws) = app.active_workspace {
                    if let Ok(manager) = SyncHistoryManager::for_workspace(&ws.name) {
                        let _ = manager.save(&app.sync_history);
                    }
                }

                // Auto-trigger status scan so dashboard is fresh
                super::backend::spawn_operation(Operation::Status, app, backend_tx.clone());
            }

            // Default to Updated filter if there were updates, else All
            app.log_filter = if wu > 0 || cl > 0 {
                LogFilter::Updated
            } else {
                LogFilter::All
            };
            app.sync_log_index = 0;

            app.operation_state = OperationState::Finished {
                operation: op,
                summary,
                with_updates: wu,
                cloned: cl,
                synced: sy,
                total_new_commits: tnc,
                duration_secs: dur,
            };
        }
        BackendMessage::OperationError(msg) => {
            app.operation_state = OperationState::Idle;
            app.error_message = Some(msg);
        }
        BackendMessage::StatusResults(entries) => {
            app.local_repos = entries;
            if matches!(
                app.operation_state,
                OperationState::Running {
                    operation: Operation::Status,
                    ..
                }
            ) {
                app.operation_state = OperationState::Idle;
            }
            app.status_loading = false;
            app.last_status_scan = Some(std::time::Instant::now());
        }
        BackendMessage::RepoCommitLog { repo_name, commits } => {
            // Only update if the user is still viewing this repo
            if app.expanded_repo.as_deref() == Some(&repo_name) {
                app.repo_commits = commits;
            }
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
