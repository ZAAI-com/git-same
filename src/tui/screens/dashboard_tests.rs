use super::*;
use crate::config::{Config, WorkspaceConfig};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tokio::sync::mpsc::unbounded_channel;

fn build_app() -> App {
    let ws = WorkspaceConfig::new("test-ws", "/tmp/test-ws");
    let mut app = App::new(Config::default(), vec![ws]);
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
