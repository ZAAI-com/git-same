use super::*;
use crate::setup::state::{OrgEntry, SetupState, SetupStep};
use crate::tui::event::{AppEvent, BackendMessage};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use git_same_core::config::{Config, WorkspaceConfig};
use git_same_core::types::OpSummary;
use tokio::sync::mpsc::unbounded_channel;

fn running_state(operation: Operation) -> OperationState {
    OperationState::Running {
        operation,
        total: 5,
        completed: 3,
        failed: 0,
        skipped: 0,
        current_repo: String::new(),
        with_updates: 0,
        cloned: 0,
        synced: 0,
        to_clone: 0,
        to_sync: 5,
        total_new_commits: 0,
        started_at: std::time::Instant::now(),
        active_repos: Vec::new(),
        throughput_samples: Vec::new(),
        last_sample_completed: 0,
    }
}

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

#[tokio::test]
async fn status_results_clears_discovering_status_state() {
    let mut app = App::new(Config::default(), Vec::new(), false);
    let (tx, _rx) = unbounded_channel();
    app.operation_state = OperationState::Discovering {
        operation: Operation::Status,
        message: "Starting Status...".to_string(),
    };
    app.status_loading = true;

    handle_event(
        &mut app,
        AppEvent::Backend(BackendMessage::StatusResults(Vec::new())),
        &tx,
    )
    .await;

    assert!(matches!(app.operation_state, OperationState::Idle));
    assert!(!app.status_loading);
    assert!(app.last_status_scan.is_some());
}

#[test]
fn status_results_clear_running_status_state() {
    let mut app = App::new(Config::default(), Vec::new(), false);
    let (tx, _rx) = unbounded_channel();
    app.operation_state = running_state(Operation::Status);
    app.status_loading = true;

    handle_backend_message(&mut app, BackendMessage::StatusResults(Vec::new()), &tx);

    assert!(matches!(app.operation_state, OperationState::Idle));
    assert!(!app.status_loading);
    assert!(app.last_status_scan.is_some());
}

#[test]
fn status_results_do_not_clear_active_sync_states() {
    let states = [
        OperationState::Discovering {
            operation: Operation::Sync,
            message: "Discovering repositories".to_string(),
        },
        running_state(Operation::Sync),
    ];

    for state in states {
        let mut app = App::new(Config::default(), Vec::new(), false);
        let (tx, _rx) = unbounded_channel();
        app.operation_state = state;
        app.status_loading = true;

        handle_backend_message(&mut app, BackendMessage::StatusResults(Vec::new()), &tx);

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
        assert!(!app.status_loading);
        assert!(app.last_status_scan.is_some());
    }
}

#[tokio::test]
async fn status_refresh_does_not_block_sync() {
    let temp = tempfile::tempdir().expect("temp workspace");
    let workspace = WorkspaceConfig::new_from_root(temp.path());
    let mut app = App::new(Config::default(), vec![workspace], false);
    let (tx, _rx) = unbounded_channel();
    app.screen = Screen::Dashboard;
    app.checks_loading = true;

    handle_event(
        &mut app,
        AppEvent::Terminal(KeyEvent::new(KeyCode::Char('t'), KeyModifiers::NONE)),
        &tx,
    )
    .await;
    assert!(matches!(app.operation_state, OperationState::Idle));
    assert!(app.status_loading);

    handle_event(
        &mut app,
        AppEvent::Backend(BackendMessage::StatusResults(Vec::new())),
        &tx,
    )
    .await;
    assert!(!app.status_loading);

    // Avoid provider/network work while still exercising the full key-routing path.
    app.active_workspace = None;
    handle_event(
        &mut app,
        AppEvent::Terminal(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE)),
        &tx,
    )
    .await;

    assert!(matches!(
        app.operation_state,
        OperationState::Discovering {
            operation: Operation::Sync,
            ..
        }
    ));
    assert!(app.error_message.is_none());
}

#[tokio::test]
async fn status_loading_tick_advances_animation_while_operation_is_idle() {
    let mut app = App::new(Config::default(), Vec::new(), false);
    let (tx, _rx) = unbounded_channel();
    app.screen = Screen::Dashboard;
    app.checks_loading = true;
    app.status_loading = true;
    app.tick_count = 9;

    handle_event(&mut app, AppEvent::Tick, &tx).await;

    assert_eq!(app.tick_count, 10);
    assert!(matches!(app.operation_state, OperationState::Idle));
}

#[tokio::test]
async fn operation_complete_starts_one_guarded_status_refresh() {
    let temp = tempfile::tempdir().expect("temp directory");
    let blocked_parent = temp.path().join("not-a-directory");
    std::fs::write(&blocked_parent, b"block workspace persistence").expect("create blocking file");
    let workspace = WorkspaceConfig::new_from_root(&blocked_parent.join("workspace"));
    let mut app = App::new(Config::default(), vec![workspace], false);
    let (tx, mut rx) = unbounded_channel();
    app.screen = Screen::Dashboard;
    app.checks_loading = true;
    app.operation_state = running_state(Operation::Sync);

    handle_backend_message(
        &mut app,
        BackendMessage::OperationComplete(OpSummary::new()),
        &tx,
    );

    assert!(matches!(
        app.operation_state,
        OperationState::Finished {
            operation: Operation::Sync,
            ..
        }
    ));
    assert!(app.status_loading);

    let tick_before = app.tick_count;
    handle_event(&mut app, AppEvent::Tick, &tx).await;
    assert_eq!(app.tick_count, tick_before.wrapping_add(1));

    let event = tokio::time::timeout(std::time::Duration::from_secs(1), rx.recv())
        .await
        .expect("post-sync status scan should complete promptly")
        .expect("backend event");
    assert!(matches!(
        event,
        AppEvent::Backend(BackendMessage::StatusResults(_))
    ));
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(100), rx.recv())
            .await
            .is_err(),
        "the following dashboard tick must not launch a duplicate status scan"
    );
}
