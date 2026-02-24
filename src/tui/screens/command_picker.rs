//! Command picker screen — select which operation to run.

use ratatui::{
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::tui::app::App;
use crate::tui::widgets::status_bar;

const COMMANDS: &[(&str, &str)] = &[
    ("Sync", "Discover repos, clone new, fetch/pull existing"),
    ("Status", "Show repository status"),
];

pub fn render(app: &App, frame: &mut Frame) {
    let chunks = Layout::vertical([
        Constraint::Length(3), // Title
        Constraint::Min(6),    // Command list
        Constraint::Length(1), // Status bar
    ])
    .split(frame.area());

    // Title
    let title = Paragraph::new(Line::from(Span::styled(
        " Select Operation ",
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )))
    .centered()
    .block(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    frame.render_widget(title, chunks[0]);

    // Command list
    let items: Vec<ListItem> = COMMANDS
        .iter()
        .enumerate()
        .map(|(i, (name, desc))| {
            let marker = if i == app.picker_index { ">" } else { " " };
            let style = if i == app.picker_index {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(Line::from(vec![
                Span::styled(format!("  {} ", marker), style),
                Span::styled(format!("{:<8}", name), style),
                Span::styled(" · ", Style::default().fg(Color::DarkGray)),
                Span::styled(*desc, Style::default().fg(Color::DarkGray)),
            ]))
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    frame.render_widget(list, chunks[1]);

    status_bar::render(frame, chunks[2], "j/k: Navigate  Enter: Run  Esc: Back");
}
