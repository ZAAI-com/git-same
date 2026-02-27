use super::*;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tokio::sync::mpsc::error::TryRecvError;

#[test]
fn wrap_comma_separated_values_wraps_and_preserves_order() {
    let values = vec![
        "CommitBook".to_string(),
        "GenAI-Wednesday".to_string(),
        "M-com".to_string(),
        "Manuel-Forks".to_string(),
    ];

    let lines = wrap_comma_separated_values(&values, 20);
    assert!(lines.len() > 1);
    assert_eq!(lines.join(", "), values.join(", "));
}

#[test]
fn wrap_comma_separated_values_empty_means_all() {
    let lines = wrap_comma_separated_values(&[], 20);
    assert_eq!(lines, vec!["all".to_string()]);
}

fn build_workspace_app(default_workspace: Option<&str>) -> App {
    let config = Config {
        default_workspace: default_workspace.map(ToString::to_string),
        ..Config::default()
    };

    let ws = WorkspaceConfig::new_from_root(std::path::Path::new("/tmp/test-ws"));
    let mut app = App::new(config, vec![ws.clone()]);
    app.screen = Screen::Workspaces;
    app.workspace_index = 0;
    app.active_workspace = Some(ws);
    app
}

#[tokio::test]
async fn workspace_key_f_opens_folder_for_selected_workspace() {
    let mut app = build_workspace_app(None);
    let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
    let _ = take_open_workspace_folder_call_count();

    handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Char('f'), KeyModifiers::NONE),
        &tx,
    )
    .await;

    assert_eq!(take_open_workspace_folder_call_count(), 1);
}

#[tokio::test]
async fn workspace_key_c_toggles_config_expansion() {
    let mut app = build_workspace_app(None);
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

    handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE),
        &tx,
    )
    .await;

    assert_eq!(app.workspace_pane, WorkspacePane::Right);
    assert!(app.settings_config_expanded);

    handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE),
        &tx,
    )
    .await;

    assert!(!app.settings_config_expanded);
    assert!(matches!(rx.try_recv(), Err(TryRecvError::Empty)));
}

#[tokio::test]
async fn workspace_left_right_controls_panel_focus_and_list_movement() {
    let config = Config::default();
    let ws1 = WorkspaceConfig::new_from_root(std::path::Path::new("/tmp/ws1"));
    let ws2 = WorkspaceConfig::new_from_root(std::path::Path::new("/tmp/ws2"));
    let mut app = App::new(config, vec![ws1.clone(), ws2]);
    app.screen = Screen::Workspaces;
    app.workspace_index = 0;
    app.active_workspace = Some(ws1);
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

    handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Right, KeyModifiers::NONE),
        &tx,
    )
    .await;
    assert_eq!(app.workspace_pane, WorkspacePane::Right);

    handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Down, KeyModifiers::NONE),
        &tx,
    )
    .await;
    assert_eq!(app.workspace_index, 0);

    handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Left, KeyModifiers::NONE),
        &tx,
    )
    .await;
    assert_eq!(app.workspace_pane, WorkspacePane::Left);

    handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Down, KeyModifiers::NONE),
        &tx,
    )
    .await;
    assert_eq!(app.workspace_index, 1);
    assert!(matches!(rx.try_recv(), Err(TryRecvError::Empty)));
}

#[tokio::test]
async fn workspace_key_o_is_noop() {
    let mut app = build_workspace_app(None);
    let before_index = app.workspace_index;
    let before_scroll = app.workspace_detail_scroll;
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let _ = take_open_workspace_folder_call_count();

    handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Char('o'), KeyModifiers::NONE),
        &tx,
    )
    .await;

    assert_eq!(app.workspace_index, before_index);
    assert_eq!(app.workspace_detail_scroll, before_scroll);
    assert_eq!(take_open_workspace_folder_call_count(), 0);
    assert!(matches!(rx.try_recv(), Err(TryRecvError::Empty)));
}

#[tokio::test]
async fn workspace_enter_selects_workspace_even_if_active() {
    let mut app = build_workspace_app(None);
    app.settings_config_expanded = true;
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

    handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
        &tx,
    )
    .await;

    assert_eq!(app.screen, Screen::Dashboard);
    assert!(matches!(rx.try_recv(), Err(TryRecvError::Empty)));
}

#[tokio::test]
async fn workspace_key_d_does_not_clear_when_already_default() {
    // The tilde_collapse_path of /tmp/test-ws should be "/tmp/test-ws" (no ~ replacement)
    let mut app = build_workspace_app(Some("/tmp/test-ws"));
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

    handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE),
        &tx,
    )
    .await;

    assert_eq!(
        app.config.default_workspace.as_deref(),
        Some("/tmp/test-ws")
    );
    assert!(matches!(rx.try_recv(), Err(TryRecvError::Empty)));
}
