use super::*;
use crate::setup::state::{PathBrowseEntry, PathSuggestion, SetupState};
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
fn render_suggestions_mode_shows_suggestions_block() {
    let mut state = SetupState::new("~/Git-Same/GitHub");
    state.path_suggestions_mode = true;
    state.path_suggestions = vec![
        PathSuggestion {
            path: "~/Git-Same/GitHub".to_string(),
            label: "current directory".to_string(),
        },
        PathSuggestion {
            path: "~/Developer".to_string(),
            label: "recommended".to_string(),
        },
    ];
    state.path_suggestion_index = 1;

    let output = render_output(&state);
    assert!(output.contains("Suggestions:"));
    assert!(output.contains("~/Developer"));
    assert!(output.contains("recommended"));
}

#[test]
fn render_browse_mode_shows_folder_navigator_context() {
    let mut state = SetupState::new("~/Git-Same/GitHub");
    state.path_suggestions_mode = false;
    state.path_browse_mode = true;
    state.path_browse_current_dir = "~/Projects".to_string();
    state.path_browse_show_hidden = false;
    state.path_browse_entries = vec![
        PathBrowseEntry {
            label: ".. (parent)".to_string(),
            path: "~".to_string(),
        },
        PathBrowseEntry {
            label: "client".to_string(),
            path: "~/Projects/client".to_string(),
        },
    ];
    state.path_browse_index = 1;

    let output = render_output(&state);
    assert!(output.contains("Folder Navigator:"));
    assert!(output.contains("~/Projects"));
    assert!(output.contains("Hidden folders: off"));
    assert!(output.contains("client"));
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
