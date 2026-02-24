//! Settings screen — application settings and quick actions.

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
    let chunks = Layout::vertical([
        Constraint::Length(3), // Title
        Constraint::Min(5),   // Content
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

    // Open Folders section
    let dim = Style::default().fg(Color::DarkGray);
    let key_style = Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD);
    let section_style = Style::default()
        .fg(Color::White)
        .add_modifier(Modifier::BOLD);

    let ws_path = app
        .active_workspace
        .as_ref()
        .map(|ws| ws.base_path.as_str())
        .unwrap_or("(no workspace selected)");

    let config_path = crate::config::Config::default_path()
        .ok()
        .and_then(|p| p.parent().map(|parent| parent.display().to_string()))
        .unwrap_or_else(|| "~/.config/git-same".to_string());

    let content = Paragraph::new(vec![
        Line::from(""),
        Line::from(Span::styled("  Open Folders", section_style)),
        Line::from(""),
        Line::from(vec![
            Span::styled("    ", dim),
            Span::styled("[c]", key_style),
            Span::styled("  Config folder", dim),
            Span::styled(format!("  — {}", config_path), dim),
        ]),
        Line::from(vec![
            Span::styled("    ", dim),
            Span::styled("[w]", key_style),
            Span::styled("  Workspace folder", dim),
            Span::styled(format!("  — {}", ws_path), dim),
        ]),
    ]);
    frame.render_widget(content, chunks[1]);

    status_bar::render(frame, chunks[2], "c: Config folder  w: Workspace folder  Esc: Back  q: Quit");
}
