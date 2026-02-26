use super::*;
use crate::config::{Config, WorkspaceConfig};
use crate::setup::state::SetupState;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tokio::sync::mpsc::unbounded_channel;

#[tokio::test]
async fn q_quits_immediately() {
    let ws = WorkspaceConfig::new("test-ws", "/tmp/test-ws");
    let mut app = App::new(Config::default(), vec![ws]);
    let (tx, _rx) = unbounded_channel();

    handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE),
        &tx,
    )
    .await;

    assert!(app.should_quit);
}

#[tokio::test]
async fn setup_cancel_returns_to_previous_screen_when_present() {
    let ws = WorkspaceConfig::new("test-ws", "/tmp/test-ws");
    let mut app = App::new(Config::default(), vec![ws]);
    app.screen = Screen::WorkspaceSetup;
    app.screen_stack = vec![Screen::SystemCheck, Screen::Workspaces];
    app.setup_state = Some(SetupState::new("~/Git-Same/GitHub"));

    handle_setup_wizard_key(&mut app, KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)).await;

    assert!(app.setup_state.is_none());
    assert_eq!(app.screen, Screen::Workspaces);
    assert_eq!(app.screen_stack, vec![Screen::SystemCheck]);
}

#[tokio::test]
async fn setup_cancel_without_history_falls_back_to_system_check() {
    let ws = WorkspaceConfig::new("test-ws", "/tmp/test-ws");
    let mut app = App::new(Config::default(), vec![ws]);
    app.screen = Screen::WorkspaceSetup;
    app.screen_stack.clear();
    app.setup_state = Some(SetupState::new("~/Git-Same/GitHub"));

    handle_setup_wizard_key(&mut app, KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)).await;

    assert!(app.setup_state.is_none());
    assert_eq!(app.screen, Screen::SystemCheck);
    assert!(app.screen_stack.is_empty());
}
