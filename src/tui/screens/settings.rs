//! Settings screen — placeholder for application settings.

use ratatui::{
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::tui::app::App;
use crate::tui::widgets::status_bar;

pub fn render(app: &App, frame: &mut Frame) {
    let _ = app;
    let chunks = Layout::vertical([
        Constraint::Length(3), // Title
        Constraint::Min(5),    // Content
        Constraint::Length(1), // Status bar
    ])
    .split(frame.area());

    let title = Paragraph::new(Line::from(vec![Span::styled(
        " Settings ",
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    )
    .centered();
    frame.render_widget(title, chunks[0]);

    let content = Paragraph::new(vec![
        Line::from(""),
        Line::from(Span::styled(
            "  Settings coming soon.",
            Style::default().fg(Color::DarkGray),
        )),
    ]);
    frame.render_widget(content, chunks[1]);

    status_bar::render(frame, chunks[2], "Esc: Back  q: Quit");
}
