use super::*;
use crate::tui::app::CheckEntry;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use git_same_core::config::{Config, WorkspaceConfig};
use ratatui::backend::TestBackend;
use ratatui::Terminal;
use tokio::sync::mpsc::unbounded_channel;

fn build_app() -> App {
    let ws = WorkspaceConfig::new_from_root(std::path::Path::new("/tmp/test-ws"));
    let mut app = App::new(Config::default(), vec![ws], false);
    app.screen = Screen::Dashboard;
    app.screen_stack.clear();
    app
}

fn build_app_in(root: &std::path::Path) -> App {
    let ws = WorkspaceConfig::new_from_root(root);
    let mut app = App::new(Config::default(), vec![ws], false);
    app.screen = Screen::Dashboard;
    app.screen_stack.clear();
    app
}

fn completed_check() -> CheckEntry {
    CheckEntry {
        name: "git".to_string(),
        passed: true,
        message: "Git is installed".to_string(),
        suggestion: None,
        critical: true,
    }
}

fn running_sync_state() -> OperationState {
    OperationState::Running {
        operation: Operation::Sync,
        total: 2,
        completed: 0,
        failed: 0,
        skipped: 0,
        current_repo: "org/repo".to_string(),
        with_updates: 0,
        cloned: 0,
        synced: 0,
        to_clone: 1,
        to_sync: 1,
        total_new_commits: 0,
        started_at: std::time::Instant::now(),
        active_repos: vec!["org/repo".to_string()],
        throughput_samples: Vec::new(),
        last_sample_completed: 0,
    }
}

fn render_output(app: &mut App) -> String {
    let backend = TestBackend::new(110, 32);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal.draw(|frame| render(app, frame)).unwrap();

    let buffer = terminal.backend().buffer();
    let mut text = String::new();
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            text.push_str(buffer[(x, y)].symbol());
        }
        text.push('\n');
    }
    text
}

#[tokio::test]
async fn t_key_does_not_set_operation_state() {
    let workspace = tempfile::tempdir().unwrap();
    let mut app = build_app_in(workspace.path());
    let (tx, _rx) = unbounded_channel();
    let completed_at = std::time::Instant::now();
    app.last_status_scan = Some(completed_at);

    handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Char('t'), KeyModifiers::NONE),
        &tx,
    )
    .await;

    assert!(matches!(app.operation_state, OperationState::Idle));
    assert!(app.status_loading);
    assert!(app.last_status_scan.is_none());
}

#[tokio::test]
async fn t_key_is_ignored_while_status_refresh_is_loading() {
    let workspace = tempfile::tempdir().unwrap();
    let mut app = build_app_in(workspace.path());
    let (tx, _rx) = unbounded_channel();
    let completed_at = std::time::Instant::now();
    app.status_loading = true;
    app.last_status_scan = Some(completed_at);
    app.check_results = vec![completed_check()];

    handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Char('t'), KeyModifiers::NONE),
        &tx,
    )
    .await;

    assert!(app.status_loading);
    assert_eq!(app.last_status_scan, Some(completed_at));
    assert_eq!(app.check_results.len(), 1);
    assert!(matches!(app.operation_state, OperationState::Idle));
}

#[tokio::test]
async fn t_key_clears_check_results() {
    let workspace = tempfile::tempdir().unwrap();
    let mut app = build_app_in(workspace.path());
    let (tx, _rx) = unbounded_channel();
    app.check_results = vec![completed_check()];

    handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Char('t'), KeyModifiers::NONE),
        &tx,
    )
    .await;

    assert!(app.check_results.is_empty());
    assert!(!app.checks_loading);
    assert!(app.status_loading);
    assert!(matches!(app.operation_state, OperationState::Idle));
}

#[tokio::test]
async fn t_key_is_ignored_while_sync_is_discovering_or_running() {
    for operation_state in [
        OperationState::Discovering {
            operation: Operation::Sync,
            message: "Discovering repositories".to_string(),
        },
        running_sync_state(),
    ] {
        let workspace = tempfile::tempdir().unwrap();
        let mut app = build_app_in(workspace.path());
        let (tx, _rx) = unbounded_channel();
        let completed_at = std::time::Instant::now();
        app.operation_state = operation_state;
        app.last_status_scan = Some(completed_at);
        app.check_results = vec![completed_check()];

        handle_key(
            &mut app,
            KeyEvent::new(KeyCode::Char('t'), KeyModifiers::NONE),
            &tx,
        )
        .await;

        assert!(!app.status_loading);
        assert_eq!(app.last_status_scan, Some(completed_at));
        assert_eq!(app.check_results.len(), 1);
        assert!(matches!(
            app.operation_state,
            OperationState::Discovering {
                operation: Operation::Sync,
                ..
            } | OperationState::Running {
                operation: Operation::Sync,
                ..
            }
        ));
    }
}

#[tokio::test]
async fn t_key_preserves_in_flight_requirement_checks() {
    let workspace = tempfile::tempdir().unwrap();
    let mut app = build_app_in(workspace.path());
    let (tx, _rx) = unbounded_channel();
    app.checks_loading = true;
    app.check_results = vec![completed_check()];

    handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Char('t'), KeyModifiers::NONE),
        &tx,
    )
    .await;

    assert!(app.status_loading);
    assert!(app.checks_loading);
    assert_eq!(app.check_results.len(), 1);
}

#[tokio::test]
async fn s_key_waits_for_active_status_refresh() {
    let workspace = tempfile::tempdir().unwrap();
    let mut app = build_app_in(workspace.path());
    let (tx, _rx) = unbounded_channel();
    app.status_loading = true;

    handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE),
        &tx,
    )
    .await;

    assert!(matches!(app.operation_state, OperationState::Idle));
    assert_eq!(
        app.error_message.as_deref(),
        Some("Status refresh is still running; try again when it completes")
    );
}

#[test]
fn status_refresh_renders_animated_spinner_instead_of_key_hint() {
    let workspace = tempfile::tempdir().unwrap();
    let mut app = build_app_in(workspace.path());
    app.status_loading = true;
    app.tick_count = 0;

    let first_frame = render_output(&mut app);
    assert!(first_frame.contains("⠋ Refreshing..."));
    assert!(!first_frame.contains("[t] Refresh"));

    app.tick_count = 1;
    let second_frame = render_output(&mut app);
    assert!(second_frame.contains("⠙ Refreshing..."));
    assert!(!second_frame.contains("[t] Refresh"));
    assert_ne!(first_frame, second_frame);
}

#[tokio::test]
async fn dashboard_s_starts_sync_without_opening_popup() {
    let mut app = build_app();
    let (tx, _rx) = unbounded_channel();

    handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE),
        &tx,
    )
    .await;

    assert_eq!(app.screen, Screen::Dashboard);
    assert!(matches!(
        app.operation_state,
        OperationState::Discovering {
            operation: Operation::Sync,
            ..
        }
    ));
}

#[tokio::test]
async fn dashboard_p_opens_sync_popup_when_idle() {
    let mut app = build_app();
    let (tx, _rx) = unbounded_channel();

    handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Char('p'), KeyModifiers::NONE),
        &tx,
    )
    .await;

    assert_eq!(app.screen, Screen::Sync);
    assert_eq!(app.screen_stack, vec![Screen::Dashboard]);
    assert!(matches!(app.operation_state, OperationState::Idle));
}

#[test]
fn hide_show_sync_progress_preserves_sync_state() {
    let mut app = build_app();
    app.scroll_offset = 9;
    app.sync_log_index = 4;

    show_sync_progress(&mut app);
    hide_sync_progress(&mut app);

    assert_eq!(app.screen, Screen::Dashboard);
    assert_eq!(app.scroll_offset, 9);
    assert_eq!(app.sync_log_index, 4);
}

#[test]
fn sync_banner_phase_is_none_when_sync_not_active() {
    let mut app = build_app();
    app.tick_count = 75;
    app.operation_state = OperationState::Discovering {
        operation: Operation::Status,
        message: "Scanning repos".to_string(),
    };

    assert_eq!(sync_banner_phase(&app), None);
}

#[test]
fn sync_banner_phase_animates_while_sync_discovering() {
    let mut app = build_app();
    app.tick_count = 75;
    app.operation_state = OperationState::Discovering {
        operation: Operation::Sync,
        message: "Discovering repos".to_string(),
    };

    let phase = sync_banner_phase(&app).expect("sync should animate the banner");
    assert!((phase - 0.5).abs() < f64::EPSILON);
}

#[test]
fn sync_banner_phase_animates_while_sync_running() {
    let mut app = build_app();
    app.tick_count = 125;
    app.operation_state = OperationState::Running {
        operation: Operation::Sync,
        total: 10,
        completed: 2,
        failed: 0,
        skipped: 0,
        current_repo: "org/repo".to_string(),
        with_updates: 1,
        cloned: 1,
        synced: 1,
        to_clone: 2,
        to_sync: 8,
        total_new_commits: 3,
        started_at: std::time::Instant::now(),
        active_repos: vec!["org/repo".to_string()],
        throughput_samples: vec![1, 1],
        last_sample_completed: 1,
    };

    let phase = sync_banner_phase(&app).expect("running sync should animate the banner");
    assert!((phase - 0.5).abs() < f64::EPSILON);
}

// --- Tests for the KeyCode::Right match-guard change introduced in this PR ---

/// Right when stat_index == 5 must NOT increment (guard: `stat_index < 5`).
#[tokio::test]
async fn right_at_last_tab_does_not_overflow() {
    let mut app = build_app();
    let (tx, _rx) = unbounded_channel();
    app.stat_index = 5;

    handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Right, KeyModifiers::NONE),
        &tx,
    )
    .await;

    assert_eq!(app.stat_index, 5, "stat_index must not exceed 5");
}

/// Right from index 4 advances to 5.
#[tokio::test]
async fn right_from_penultimate_tab_advances_to_last() {
    let mut app = build_app();
    let (tx, _rx) = unbounded_channel();
    app.stat_index = 4;

    handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Right, KeyModifiers::NONE),
        &tx,
    )
    .await;

    assert_eq!(app.stat_index, 5);
}

/// Right from index 0 advances to 1.
#[tokio::test]
async fn right_from_first_tab_advances_to_second() {
    let mut app = build_app();
    let (tx, _rx) = unbounded_channel();
    app.stat_index = 0;

    handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Right, KeyModifiers::NONE),
        &tx,
    )
    .await;

    assert_eq!(app.stat_index, 1);
}

/// Left at stat_index == 0 must not underflow (saturating_sub keeps it at 0).
#[tokio::test]
async fn left_at_first_tab_stays_at_zero() {
    let mut app = build_app();
    let (tx, _rx) = unbounded_channel();
    app.stat_index = 0;

    handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Left, KeyModifiers::NONE),
        &tx,
    )
    .await;

    assert_eq!(app.stat_index, 0);
}

/// Left from index 3 decrements to 2.
#[tokio::test]
async fn left_from_middle_tab_decrements() {
    let mut app = build_app();
    let (tx, _rx) = unbounded_channel();
    app.stat_index = 3;

    handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Left, KeyModifiers::NONE),
        &tx,
    )
    .await;

    assert_eq!(app.stat_index, 2);
}

/// Right resets the table selection to row 0.
#[tokio::test]
async fn right_resets_table_selection_to_first_row() {
    let mut app = build_app();
    let (tx, _rx) = unbounded_channel();
    app.stat_index = 2;
    app.dashboard_table_state.select(Some(7));

    handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Right, KeyModifiers::NONE),
        &tx,
    )
    .await;

    assert_eq!(app.stat_index, 3);
    assert_eq!(app.dashboard_table_state.selected(), Some(0));
}

/// Left resets the table selection to row 0.
#[tokio::test]
async fn left_resets_table_selection_to_first_row() {
    let mut app = build_app();
    let (tx, _rx) = unbounded_channel();
    app.stat_index = 4;
    app.dashboard_table_state.select(Some(5));

    handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Left, KeyModifiers::NONE),
        &tx,
    )
    .await;

    assert_eq!(app.stat_index, 3);
    assert_eq!(app.dashboard_table_state.selected(), Some(0));
}
