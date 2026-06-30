use super::*;
use crate::setup::state::{SetupState, SetupStep};
use ratatui::backend::TestBackend;
use ratatui::Terminal;

fn render_output(state: &SetupState, width: u16, height: u16) -> String {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal.draw(|frame| render(state, frame)).unwrap();

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
fn center_cell_matches_width() {
    let out = center_cell("Auth", 10);
    assert_eq!(out.chars().count(), 10);
    assert!(out.contains("Auth"));
}

#[test]
fn connector_cell_matches_width() {
    assert_eq!(connector_cell(7, true).chars().count(), 7);
    assert_eq!(connector_cell(7, false).chars().count(), 7);
}

#[test]
fn render_keeps_banner_visible_while_path_popup_is_open() {
    let mut state = SetupState::new("~/Git-Same/GitHub");
    state.step = SetupStep::SelectPath;
    state.path_browse_mode = true;

    let output = render_output(&state, 120, 40);
    assert!(output.contains("Local Folder Navigator"));
    assert!(output.contains("██████╗"));
}
