use super::*;

#[test]
fn test_new_no_workspaces_shows_setup_wizard() {
    let app = App::new(Config::default(), vec![]);
    assert_eq!(app.screen, Screen::WorkspaceSetup);
    assert!(app.setup_state.is_some());
    assert!(app.active_workspace.is_none());
    assert!(app.base_path.is_none());
}

#[test]
fn test_new_single_workspace_auto_selects() {
    let ws = WorkspaceConfig::new_from_root(std::path::Path::new("/tmp/test"));
    let app = App::new(Config::default(), vec![ws]);
    assert_eq!(app.screen, Screen::Dashboard);
    assert!(app.active_workspace.is_some());
    assert_eq!(
        app.active_workspace.unwrap().root_path,
        std::path::PathBuf::from("/tmp/test")
    );
    assert!(app.base_path.is_some());
}

#[test]
fn test_new_multiple_no_default_shows_selector() {
    let ws1 = WorkspaceConfig::new_from_root(std::path::Path::new("/tmp/ws1"));
    let ws2 = WorkspaceConfig::new_from_root(std::path::Path::new("/tmp/ws2"));
    let app = App::new(Config::default(), vec![ws1, ws2]);
    assert_eq!(app.screen, Screen::Workspaces);
    assert!(app.active_workspace.is_none());
}

#[test]
fn test_new_multiple_with_valid_default_auto_selects() {
    let ws1 = WorkspaceConfig::new_from_root(std::path::Path::new("/tmp/ws1"));
    let ws2 = WorkspaceConfig::new_from_root(std::path::Path::new("/tmp/ws2"));
    let config = Config {
        default_workspace: Some("/tmp/ws2".to_string()),
        ..Config::default()
    };
    let app = App::new(config, vec![ws1, ws2]);
    assert_eq!(app.screen, Screen::Dashboard);
    assert_eq!(
        app.active_workspace.unwrap().root_path,
        std::path::PathBuf::from("/tmp/ws2")
    );
}

#[test]
fn test_new_multiple_with_invalid_default_shows_selector() {
    let ws1 = WorkspaceConfig::new_from_root(std::path::Path::new("/tmp/ws1"));
    let ws2 = WorkspaceConfig::new_from_root(std::path::Path::new("/tmp/ws2"));
    let config = Config {
        default_workspace: Some("/tmp/nonexistent".to_string()),
        ..Config::default()
    };
    let app = App::new(config, vec![ws1, ws2]);
    assert_eq!(app.screen, Screen::Workspaces);
    assert!(app.active_workspace.is_none());
}
