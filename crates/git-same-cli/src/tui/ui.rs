//! Top-level rendering dispatcher (the "View").

use super::app::{App, Screen};
use super::screens;
use ratatui::Frame;

/// Render the current screen.
pub fn render(app: &mut App, frame: &mut Frame) {
    match app.screen {
        Screen::WorkspaceSetup => {
            if let Some(ref setup) = app.setup_state {
                crate::setup::ui::render(setup, frame);
            }
        }
        Screen::Workspaces => screens::workspaces::render(app, frame),
        Screen::Dashboard => screens::dashboard::render(app, frame),
        Screen::Sync => {
            screens::dashboard::render(app, frame);
            screens::sync::render(app, frame);
        }
        Screen::Settings => screens::settings::render(app, frame),
    }
}
