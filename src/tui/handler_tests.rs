use super::*;
use crate::config::{Config, WorkspaceConfig};
use crate::setup::state::{OrgEntry, SetupState, SetupStep};
use crate::tui::event::{AppEvent, BackendMessage};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tokio::sync::mpsc::unbounded_channel;

#[tokio::test]
async fn q_quits_immediately() {
    let ws = WorkspaceConfig::new_from_root(std::path::Path::new("/tmp/test-ws"));
    let mut app = App::new(Config::default(), vec![ws], false);
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
    let ws = WorkspaceConfig::new_from_root(std::path::Path::new("/tmp/test-ws"));
    let mut app = App::new(Config::default(), vec![ws], false);
    app.screen = Screen::WorkspaceSetup;
    app.screen_stack = vec![Screen::Settings, Screen::Workspaces];
    let mut setup = SetupState::new("~/Git-Same/GitHub");
    setup.step = crate::setup::state::SetupStep::SelectProvider;
    app.setup_state = Some(setup);

    handle_setup_wizard_key(&mut app, KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)).await;

    assert!(app.setup_state.is_none());
    assert_eq!(app.screen, Screen::Workspaces);
    assert_eq!(app.screen_stack, vec![Screen::Settings]);
}

#[tokio::test]
async fn setup_cancel_without_history_quits() {
    let ws = WorkspaceConfig::new_from_root(std::path::Path::new("/tmp/test-ws"));
    let mut app = App::new(Config::default(), vec![ws], false);
    app.screen = Screen::WorkspaceSetup;
    app.screen_stack.clear();
    let mut setup = SetupState::new("~/Git-Same/GitHub");
    setup.step = crate::setup::state::SetupStep::SelectProvider;
    app.setup_state = Some(setup);

    handle_setup_wizard_key(&mut app, KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)).await;

    assert!(app.setup_state.is_none());
    assert!(app.should_quit);
}

#[tokio::test]
async fn setup_right_moves_to_next_step() {
    let ws = WorkspaceConfig::new_from_root(std::path::Path::new("/tmp/test-ws"));
    let mut app = App::new(Config::default(), vec![ws], false);
    app.screen = Screen::WorkspaceSetup;
    let mut setup = SetupState::new("~/Git-Same/GitHub");
    setup.step = SetupStep::SelectProvider;
    app.setup_state = Some(setup);

    handle_setup_wizard_key(&mut app, KeyEvent::new(KeyCode::Right, KeyModifiers::NONE)).await;

    assert_eq!(
        app.setup_state.as_ref().map(|s| s.step),
        Some(SetupStep::Authenticate)
    );
}

#[tokio::test]
async fn setup_org_discovery_backend_message_populates_state() {
    let ws = WorkspaceConfig::new_from_root(std::path::Path::new("/tmp/test-ws"));
    let mut app = App::new(Config::default(), vec![ws], false);
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

#[tokio::test]
async fn setup_check_results_preserve_suggestions() {
    let ws = WorkspaceConfig::new_from_root(std::path::Path::new("/tmp/test-ws"));
    let mut app = App::new(Config::default(), vec![ws], false);
    let (tx, _rx) = unbounded_channel();
    app.screen = Screen::WorkspaceSetup;
    app.setup_state = Some(SetupState::new("~/Git-Same/GitHub"));

    handle_event(
        &mut app,
        AppEvent::Backend(BackendMessage::SetupCheckResults(vec![CheckEntry {
            name: "gh".to_string(),
            passed: false,
            message: "Not authenticated".to_string(),
            suggestion: Some("Run: gh auth login".to_string()),
            critical: true,
        }])),
        &tx,
    )
    .await;

    assert_eq!(app.check_results.len(), 1);
    assert_eq!(
        app.check_results[0].suggestion.as_deref(),
        Some("Run: gh auth login")
    );

    let setup = app.setup_state.as_ref().expect("setup state");
    assert_eq!(setup.check_results.len(), 1);
    assert_eq!(
        setup.check_results[0].suggestion.as_deref(),
        Some("Run: gh auth login")
    );
}
