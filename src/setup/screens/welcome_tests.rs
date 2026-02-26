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
fn render_welcome_shows_intro_and_steps() {
    let state = SetupState::with_first_setup("~/Git-Same/GitHub", true);

    let output = render_output(&state);
    assert!(output.contains("Welcome to Git-Same"));
    assert!(output.contains("Connect to your Git provider"));
    assert!(output.contains("Authenticate your account"));
    assert!(output.contains("Press Enter to get started"));
}
