use super::*;
use crate::setup::state::{OrgEntry, SetupState};
use ratatui::backend::TestBackend;
use ratatui::Terminal;

fn render_output(state: &SetupState) -> String {
    let backend = TestBackend::new(100, 24);
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
fn render_loading_state_shows_discovery_message() {
    let mut state = SetupState::new("~/Git-Same/GitHub");
    state.org_loading = true;
    state.tick_count = 3;

    let output = render_output(&state);
    assert!(output.contains("Select organizations to sync"));
    assert!(output.contains("Discovering organizations"));
}

#[test]
fn render_populated_orgs_shows_selection_summary() {
    let mut state = SetupState::new("~/Git-Same/GitHub");
    state.org_loading = false;
    state.orgs = vec![
        OrgEntry {
            name: "acme".to_string(),
            repo_count: 5,
            selected: true,
        },
        OrgEntry {
            name: "beta".to_string(),
            repo_count: 10,
            selected: false,
        },
    ];
    state.org_index = 0;

    let output = render_output(&state);
    assert!(output.contains("1 of 2 selected"));
    assert!(output.contains("5 repos"));
    assert!(output.contains("acme"));
    assert!(output.contains("beta"));
}

#[test]
fn render_empty_orgs_shows_personal_repo_hint() {
    let state = SetupState::new("~/Git-Same/GitHub");

    let output = render_output(&state);
    assert!(output.contains("No organizations found"));
    assert!(output.contains("personal repos"));
}
