use super::*;
use crate::setup::state::{OrgEntry, SetupState};
use ratatui::backend::TestBackend;
use ratatui::Terminal;

fn render_output(state: &SetupState) -> String {
    let backend = TestBackend::new(110, 24);
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
fn render_confirm_shows_workspace_summary() {
    let mut state = SetupState::new("~/Git-Same/GitHub");
    state.username = Some("octocat".to_string());
    state.workspace_name = "personal-workspace".to_string();
    state.orgs = vec![
        OrgEntry {
            name: "acme".to_string(),
            repo_count: 4,
            selected: true,
        },
        OrgEntry {
            name: "tools".to_string(),
            repo_count: 2,
            selected: true,
        },
    ];

    let output = render_output(&state);
    assert!(output.contains("Review Workspace Configuration"));
    assert!(output.contains("@octocat"));
    assert!(output.contains("personal-workspace"));
    assert!(output.contains("acme, tools"));
}

#[test]
fn render_confirm_shows_inline_error_when_present() {
    let mut state = SetupState::new("~/Git-Same/GitHub");
    state.workspace_name = "broken".to_string();
    state.error_message = Some("Unable to write config".to_string());

    let output = render_output(&state);
    assert!(output.contains("Press Enter to save and continue"));
    assert!(output.contains("Error: Unable to write config"));
}
