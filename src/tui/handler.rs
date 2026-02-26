//! Input handler: keyboard events → state mutations (the "Update").

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tokio::sync::mpsc::UnboundedSender;

use super::app::{
    App, CheckEntry, LogFilter, Operation, OperationState, Screen, SyncHistoryEntry, SyncLogEntry,
    SyncLogStatus,
};
use super::event::{AppEvent, BackendMessage};
use super::screens;
use crate::cache::SyncHistoryManager;
use crate::config::WorkspaceManager;
use crate::domain::RepoPathTemplate;
use crate::setup::state::{SetupOutcome, SetupStep};

const MAX_THROUGHPUT_SAMPLES: usize = 240;
const MAX_LOG_LINES: usize = 5_000;

/// Handle an incoming event, updating app state and optionally spawning backend work.
pub async fn handle_event(app: &mut App, event: AppEvent, backend_tx: &UnboundedSender<AppEvent>) {
    match event {
        AppEvent::Terminal(key) => handle_key(app, key, backend_tx).await,
        AppEvent::Backend(msg) => handle_backend_message(app, msg, backend_tx),
        AppEvent::Tick => {
            let sync_in_progress = matches!(
                &app.operation_state,
                OperationState::Discovering {
                    operation: Operation::Sync,
                    ..
                } | OperationState::Running {
                    operation: Operation::Sync,
                    ..
                }
            );

            // Keep sync animation/throughput sampling active even when progress popup is hidden.
            if sync_in_progress {
                app.tick_count = app.tick_count.wrapping_add(1);

                // Sample throughput every 10 ticks (1 second at 100ms tick rate)
                if app.tick_count.is_multiple_of(10) {
                    if let OperationState::Running {
                        operation: Operation::Sync,
                        completed,
                        ref mut throughput_samples,
                        ref mut last_sample_completed,
                        ..
                    } = app.operation_state
                    {
                        let delta = completed.saturating_sub(*last_sample_completed) as u64;
                        throughput_samples.push(delta);
                        if throughput_samples.len() > MAX_THROUGHPUT_SAMPLES {
                            let drop_count = throughput_samples.len() - MAX_THROUGHPUT_SAMPLES;
                            throughput_samples.drain(0..drop_count);
                        }
                        *last_sample_completed = completed;
                    }
                }
            }
            // Drive setup wizard tick and org discovery on tick
            if app.screen == Screen::WorkspaceSetup {
                if let Some(ref mut setup) = app.setup_state {
                    setup.tick_count = setup.tick_count.wrapping_add(1);
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
                && !sync_in_progress
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

    // Global keybindings
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        app.should_quit = true;
        return;
    }

    if key.code == KeyCode::Char('q') {
        app.should_quit = true;
        return;
    }

    // WorkspaceSetup handles its own screen-specific keys.
    if app.screen == Screen::WorkspaceSetup {
        handle_setup_wizard_key(app, key).await;
        return;
    }

    if key.code == KeyCode::Esc {
        // On Sync screen, collapse expanded entry before navigating back
        if app.screen == Screen::Sync && app.expanded_repo.is_some() {
            app.expanded_repo = None;
            app.repo_commits.clear();
            return;
        }
        // Ensure Sync can always minimize back to Dashboard.
        if app.screen == Screen::Sync && app.screen_stack.is_empty() {
            app.screen = Screen::Dashboard;
            return;
        }
        // Don't go back from entry points (no screen stack)
        if app.screen == Screen::SystemCheck {
            return;
        }
        // Workspace screen: only go back if navigated to (has screen stack)
        if app.screen == Screen::Workspaces && app.screen_stack.is_empty() {
            return;
        }
        app.go_back();
        return;
    }

    // Screen-specific keybindings
    match app.screen {
        Screen::SystemCheck => screens::system_check::handle_key(app, key, backend_tx).await,
        Screen::WorkspaceSetup => unreachable!(), // handled above
        Screen::Workspaces => screens::workspaces::handle_key(app, key, backend_tx).await,
        Screen::Dashboard => screens::dashboard::handle_key(app, key, backend_tx).await,
        Screen::Sync => screens::sync::handle_key(app, key, backend_tx),
        Screen::Settings => screens::settings::handle_key(app, key),
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
            // Cancelled — return to previous screen when available.
            app.setup_state = None;
            if app.screen_stack.is_empty() {
                app.screen = Screen::SystemCheck;
            } else {
                app.go_back();
            }
        }
    }
}

/// Compute the filesystem path for a repo from its full name (e.g. "org/repo").
/// Mirrors `DiscoveryOrchestrator::compute_path()` logic using workspace config.
fn compute_repo_path(app: &App, repo_name: &str) -> Option<std::path::PathBuf> {
    let ws = app.active_workspace.as_ref()?;
    let base_path = ws.expanded_base_path();
    let template = ws
        .structure
        .clone()
        .unwrap_or_else(|| app.config.structure.clone());
    let provider_name = ws.provider.kind.to_string().to_lowercase();

    RepoPathTemplate::new(template).render_full_name(&base_path, &provider_name, repo_name)
}

fn handle_backend_message(
    app: &mut App,
    msg: BackendMessage,
    backend_tx: &UnboundedSender<AppEvent>,
) {
    match msg {
        BackendMessage::OrgsDiscovered(count) => {
            app.operation_state = OperationState::Discovering {
                operation: Operation::Sync,
                message: format!("Found {} organizations", count),
            };
        }
        BackendMessage::OrgStarted(name) => {
            app.operation_state = OperationState::Discovering {
                operation: Operation::Sync,
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

            if app.log_lines.len() >= MAX_LOG_LINES {
                let drop_count = app.log_lines.len() + 1 - MAX_LOG_LINES;
                app.log_lines.drain(0..drop_count);
                app.scroll_offset = app.scroll_offset.saturating_sub(drop_count);
            }
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
            // Single repo deep dive (Enter key)
            if app.expanded_repo.as_deref() == Some(&repo_name) {
                app.repo_commits = commits.clone();
            }
            // Changelog aggregation (c key)
            if app.log_filter == LogFilter::Changelog {
                app.changelog_commits.insert(repo_name, commits);
                app.changelog_loaded += 1;
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

#[cfg(test)]
#[path = "handler_tests.rs"]
mod tests;
