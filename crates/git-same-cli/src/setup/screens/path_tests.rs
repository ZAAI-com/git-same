use super::*;
use crate::setup::state::{PathBrowseEntry, SetupState};
use ratatui::backend::TestBackend;
use ratatui::Terminal;

fn render_output(state: &SetupState) -> String {
    let backend = TestBackend::new(90, 26);
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
fn render_path_input_shows_base_path_without_suggestions() {
    let mut state = SetupState::new("~/Git-Same/GitHub");
    state.path_suggestions_mode = false;
    state.path_browse_mode = false;

    let output = render_output(&state);
    assert!(output.contains("Base Path"));
    assert!(output.contains("~/Git-Same/GitHub"));
    assert!(!output.contains("Suggestions:"));
}

#[test]
fn render_browse_mode_shows_folder_navigator_context() {
    let mut state = SetupState::new("~/Git-Same/GitHub");
    state.path_suggestions_mode = false;
    state.path_browse_mode = true;
    state.path_browse_current_dir = "~/Projects".to_string();
    state.path_browse_entries = vec![
        PathBrowseEntry {
            label: "Projects/".to_string(),
            path: "~/Projects".to_string(),
            depth: 0,
            expanded: true,
            has_children: true,
        },
        PathBrowseEntry {
            label: "client/".to_string(),
            path: "~/Projects/client".to_string(),
            depth: 1,
            expanded: false,
            has_children: false,
        },
    ];
    state.path_browse_index = 1;

    let output = render_output(&state);
    assert!(output.contains("Local Folder Navigator"));
    assert!(output.contains("Path:"));
    assert!(output.contains("~/Projects"));
    assert!(output.contains("client"));
    assert!(output.contains("[Esc] Close"));
    assert!(output.contains("[Enter] Select"));
}

#[test]
fn render_error_state_shows_preview_and_error_message() {
    let mut state = SetupState::new("~/Git-Same/GitHub");
    state.path_suggestions_mode = false;
    state.path_browse_mode = false;
    state.base_path = "~/invalid-path".to_string();
    state.path_cursor = state.base_path.len();
    state.error_message = Some("Path does not exist".to_string());

    let output = render_output(&state);
    assert!(output.contains("Preview:"));
    assert!(output.contains("~/invalid-path/acme-corp/my-repo/"));
    assert!(output.contains("Path does not exist"));
}
