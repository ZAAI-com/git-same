//! Top-level rendering dispatcher (the "View").

use super::app::{App, Screen};
use super::screens;
use ratatui::Frame;

/// Render the current screen.
pub fn render(app: &App, frame: &mut Frame) {
    match app.screen {
        Screen::InitCheck => screens::init_check::render(app, frame),
        Screen::WorkspaceSelector => screens::workspace_selector::render(app, frame),
        Screen::Dashboard => screens::dashboard::render(app, frame),
        Screen::CommandPicker => screens::command_picker::render(app, frame),
        Screen::OrgBrowser => screens::org_browser::render(app, frame),
        Screen::Progress => screens::progress::render(app, frame),
        Screen::RepoStatus => screens::repo_status::render(app, frame),
    }
}
