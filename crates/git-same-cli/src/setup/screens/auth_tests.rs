use super::*;
use crate::setup::state::{AuthStatus, SetupState};
use ratatui::backend::TestBackend;
use ratatui::Terminal;

fn render_output(state: &SetupState) -> String {
    let backend = TestBackend::new(100, 22);
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
fn render_pending_state_prompts_for_authentication() {
    let mut state = SetupState::new("~/Git-Same/GitHub");
    state.auth_status = AuthStatus::Pending;

    let output = render_output(&state);
    assert!(output.contains("Authenticate with"));
    assert!(output.contains("Press Enter to authenticate"));
}

#[test]
fn render_success_state_shows_username_and_continue_hint() {
    let mut state = SetupState::new("~/Git-Same/GitHub");
    state.auth_status = AuthStatus::Success;
    state.username = Some("octocat".to_string());

    let output = render_output(&state);
    assert!(output.contains("Authenticated"));
    assert!(output.contains("@octocat"));
    assert!(output.contains("Press Enter to continue"));
}

#[test]
fn render_failed_state_shows_error_guidance() {
    let mut state = SetupState::new("~/Git-Same/GitHub");
    state.auth_status = AuthStatus::Failed("token missing".to_string());

    let output = render_output(&state);
    assert!(output.contains("Authentication failed"));
    assert!(output.contains("token missing"));
    assert!(output.contains("gh auth login"));
}
