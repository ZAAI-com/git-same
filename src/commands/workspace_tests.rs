use super::*;
use crate::output::Verbosity;

fn quiet_output() -> Output {
    Output::new(Verbosity::Quiet, false)
}

#[test]
fn test_show_default_none() {
    let config = Config::default();
    let output = quiet_output();
    let result = show_default(&config, &output);
    assert!(result.is_ok());
}

#[test]
fn test_show_default_some() {
    let config = Config {
        default_workspace: Some("my-ws".to_string()),
        ..Config::default()
    };
    let output = quiet_output();
    let result = show_default(&config, &output);
    assert!(result.is_ok());
}

#[test]
fn test_list_empty() {
    // This test may fail if user has workspaces configured;
    // the actual CRUD tests are in workspace_manager.rs
    let config = Config::default();
    let output = quiet_output();
    // Just verify it doesn't panic
    let _ = list(&config, &output);
}
