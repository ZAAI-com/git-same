use super::*;
use crate::config::{Config, WorkspaceConfig};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tokio::sync::mpsc::unbounded_channel;

fn build_app() -> App {
    let ws = WorkspaceConfig::new_from_root(std::path::Path::new("/tmp/test-ws"));
    let mut app = App::new(Config::default(), vec![ws], false);
    app.screen = Screen::Dashboard;
    app.screen_stack.clear();
    app
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
