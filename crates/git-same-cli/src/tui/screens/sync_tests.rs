use super::*;
use crate::tui::app::{Operation, Screen};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use git_same_core::config::{Config, WorkspaceConfig};
use git_same_core::types::OpSummary;
use tokio::sync::mpsc::unbounded_channel;

fn build_app() -> App {
    let ws = WorkspaceConfig::new_from_root(std::path::Path::new("/tmp/test-ws"));
    let mut app = App::new(Config::default(), vec![ws], false);
    app.screen = Screen::Sync;
    app.screen_stack = vec![Screen::Dashboard];
    app
}

#[test]
fn sync_key_p_hides_progress_popup() {
    let mut app = build_app();
    let (tx, _rx) = unbounded_channel();
    app.scroll_offset = 5;

    handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Char('p'), KeyModifiers::NONE),
        &tx,
    );

    assert_eq!(app.screen, Screen::Dashboard);
    assert_eq!(app.scroll_offset, 5);
}

#[tokio::test]
async fn sync_key_s_starts_sync() {
    let mut app = build_app();
    let (tx, _rx) = unbounded_channel();

    handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE),
        &tx,
    );

    assert_eq!(app.screen, Screen::Sync);
    assert!(matches!(
        app.operation_state,
        OperationState::Discovering {
            operation: Operation::Sync,
            ..
        }
    ));
}

#[test]
fn right_arrow_cycles_finished_filter() {
    let mut app = build_app();
    let (tx, _rx) = unbounded_channel();
    app.operation_state = OperationState::Finished {
        operation: Operation::Sync,
        summary: OpSummary {
            success: 1,
            failed: 0,
            skipped: 0,
        },
        with_updates: 0,
        cloned: 0,
        synced: 1,
        total_new_commits: 0,
        duration_secs: 1.0,
    };
    app.log_filter = LogFilter::All;

    handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Right, KeyModifiers::NONE),
        &tx,
    );

    assert_eq!(app.log_filter, LogFilter::Updated);
}
