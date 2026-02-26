//! Step 0: Welcome screen (first-time setup only).

use crate::setup::state::SetupState;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

pub fn render(_state: &SetupState, frame: &mut Frame, area: Rect) {
    let chunks = Layout::vertical([
        Constraint::Length(3), // Title
        Constraint::Min(10),   // Content
        Constraint::Length(2), // Help
    ])
    .split(area);

    // Title
    let title = Paragraph::new("Welcome to Git-Same").style(
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    );
    frame.render_widget(title, chunks[0]);

    // Content
    let cyan = Style::default().fg(Color::Cyan);
    let dim = Style::default().fg(Color::DarkGray);
    let white = Style::default().fg(Color::White);

    let lines = vec![
        Line::raw(""),
        Line::from(Span::styled(
            "  Git-Same mirrors your GitHub organization structure",
            white,
        )),
        Line::from(Span::styled(
            "  to your local file system with parallel cloning",
            white,
        )),
        Line::from(Span::styled("  and syncing.", white)),
        Line::raw(""),
        Line::from(Span::styled("  This wizard will help you:", dim)),
        Line::raw(""),
        Line::from(vec![
            Span::styled("    1. ", cyan),
            Span::styled("Connect to your Git provider", white),
        ]),
        Line::from(vec![
            Span::styled("    2. ", cyan),
            Span::styled("Authenticate your account", white),
        ]),
        Line::from(vec![
            Span::styled("    3. ", cyan),
            Span::styled("Select which organizations to sync", white),
        ]),
        Line::from(vec![
            Span::styled("    4. ", cyan),
            Span::styled("Choose where to store repos", white),
        ]),
        Line::raw(""),
        Line::from(Span::styled(
            "  Press Enter to get started",
            Style::default().fg(Color::Yellow),
        )),
    ];

    let content = Paragraph::new(lines);
    frame.render_widget(content, chunks[1]);

    // Help
    let help =
        Paragraph::new("Enter Start  Esc Cancel").style(Style::default().fg(Color::DarkGray));
    frame.render_widget(help, chunks[2]);
}

#[cfg(test)]
#[path = "welcome_tests.rs"]
mod tests;
