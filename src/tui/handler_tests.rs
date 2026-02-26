use super::*;
use crate::config::{Config, WorkspaceConfig};
use crate::setup::state::{OrgEntry, SetupState, SetupStep};
use crate::tui::event::{AppEvent, BackendMessage};
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

#[tokio::test]
async fn setup_right_moves_to_next_step() {
    let ws = WorkspaceConfig::new("test-ws", "/tmp/test-ws");
    let mut app = App::new(Config::default(), vec![ws]);
    app.screen = Screen::WorkspaceSetup;
    app.setup_state = Some(SetupState::new("~/Git-Same/GitHub"));

    handle_setup_wizard_key(&mut app, KeyEvent::new(KeyCode::Right, KeyModifiers::NONE)).await;

    assert_eq!(
        app.setup_state.as_ref().map(|s| s.step),
        Some(SetupStep::Authenticate)
    );
}

#[tokio::test]
async fn setup_org_discovery_backend_message_populates_state() {
    let ws = WorkspaceConfig::new("test-ws", "/tmp/test-ws");
    let mut app = App::new(Config::default(), vec![ws]);
    let (tx, _rx) = unbounded_channel();
    app.screen = Screen::WorkspaceSetup;
    app.setup_state = Some(SetupState::new("~/Git-Same/GitHub"));
    let setup = app.setup_state.as_mut().expect("setup state");
    setup.step = SetupStep::SelectOrgs;
    setup.org_loading = true;
    setup.org_discovery_in_progress = true;

    handle_event(
        &mut app,
        AppEvent::Backend(BackendMessage::SetupOrgsDiscovered(vec![OrgEntry {
            name: "acme".to_string(),
            repo_count: 3,
            selected: true,
        }])),
        &tx,
    )
    .await;

    let setup = app.setup_state.as_ref().expect("setup state");
    assert!(!setup.org_loading);
    assert!(!setup.org_discovery_in_progress);
    assert!(setup.org_error.is_none());
    assert_eq!(setup.orgs.len(), 1);
    assert_eq!(setup.orgs[0].name, "acme");
}
