use super::*;
use crate::setup::state::{OrgEntry, SetupState};
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
fn render_complete_first_setup_shows_workspace_created() {
    let mut state = SetupState::with_first_setup("~/Git-Same/GitHub", true);
    state.base_path = "~/Git-Same/GitHub".to_string();
    state.orgs = vec![OrgEntry {
        name: "acme".to_string(),
        repo_count: 12,
        selected: true,
    }];

    let output = render_output(&state);
    assert!(output.contains("Workspace Created!"));
    assert!(output.contains("1 organization"));
    assert!(output.contains("12 repos"));
}

#[test]
fn render_complete_additional_setup_shows_workspace_added() {
    let mut state = SetupState::with_first_setup("~/Git-Same/GitHub", false);
    state.base_path = "~/Git-Same/GitHub".to_string();

    let output = render_output(&state);
    assert!(output.contains("Workspace Added!"));
    assert!(output.contains("Press Enter to continue"));
}
