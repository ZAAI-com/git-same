use super::*;
use crate::setup::state::SetupState;
use ratatui::backend::TestBackend;
use ratatui::Terminal;

fn render_output(state: &SetupState) -> String {
    let backend = TestBackend::new(100, 20);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|frame| {
            let area = frame.area();
            render(state, frame, area);
        })
        .unwrap();

    let buffer = terminal.backend().buffer();
    let mut text = String::new();
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            text.push_str(buffer[(x, y)].symbol());
        }
        text.push('\n');
    }
    text
}

#[test]
fn render_first_setup_shows_welcome_title() {
    let state = SetupState::with_first_setup("~/Git-Same/GitHub", true);
    let output = render_output(&state);
    assert!(output.contains("Welcome to Git-Same"));
}

#[test]
fn render_non_first_setup_shows_requirements_title() {
    let state = SetupState::with_first_setup("~/Git-Same/GitHub", false);
    let output = render_output(&state);
    assert!(output.contains("System Requirements"));
}

#[test]
fn render_loading_shows_spinner() {
    let mut state = SetupState::new("~/Git-Same/GitHub");
    state.checks_loading = true;
    let output = render_output(&state);
    assert!(output.contains("Checking requirements"));
}

#[test]
fn render_passed_checks_shows_continue_hint() {
    let mut state = SetupState::new("~/Git-Same/GitHub");
    state.checks_loading = false;
    state.check_results = vec![crate::checks::CheckResult {
        name: "Git".to_string(),
        passed: true,
        message: "git 2.43.0".to_string(),
        suggestion: None,
        critical: true,
    }];
    let output = render_output(&state);
    assert!(output.contains("All requirements met"));
}

#[test]
fn render_failed_critical_shows_fix_hint() {
    let mut state = SetupState::new("~/Git-Same/GitHub");
    state.checks_loading = false;
    state.check_results = vec![crate::checks::CheckResult {
        name: "Git".to_string(),
        passed: false,
        message: "not found".to_string(),
        suggestion: None,
        critical: true,
    }];
    let output = render_output(&state);
    assert!(output.contains("Fix critical requirements"));
}

#[test]
fn render_config_created_shows_created_label() {
    let mut state = SetupState::new("~/Git-Same/GitHub");
    state.config_was_created = true;
    state.config_path_display = Some("~/.config/git-same/config.toml".to_string());
    let output = render_output(&state);
    assert!(output.contains("Config created at"));
}

#[test]
fn render_config_found_shows_found_label() {
    let mut state = SetupState::new("~/Git-Same/GitHub");
    state.config_was_created = false;
    state.config_path_display = Some("~/.config/git-same/config.toml".to_string());
    let output = render_output(&state);
    assert!(output.contains("Config found at"));
}
