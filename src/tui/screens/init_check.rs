//! Init check screen — displays requirement check results.

use ratatui::{
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::tui::app::App;
use crate::tui::widgets::status_bar;

pub fn render(app: &App, frame: &mut Frame) {
    let chunks = Layout::vertical([
        Constraint::Length(3), // Title
        Constraint::Min(8),    // Check results
        Constraint::Length(3), // Help
        Constraint::Length(1), // Status bar
    ])
    .split(frame.area());

    // Title
    let title = Paragraph::new(Line::from(Span::styled(
        " System Requirements ",
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

    // Check results
    if app.checks_loading {
        let loading = Paragraph::new(Line::from(Span::styled(
            "  Checking requirements...",
            Style::default().fg(Color::Yellow),
        )))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        );
        frame.render_widget(loading, chunks[1]);
    } else if app.check_results.is_empty() {
        let empty = Paragraph::new(Line::from(Span::styled(
            "  Press Enter to check requirements",
            Style::default().fg(Color::DarkGray),
        )))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        );
        frame.render_widget(empty, chunks[1]);
    } else {
        let items: Vec<ListItem> = app
            .check_results
            .iter()
            .map(|check| {
                let (icon, color) = if check.passed {
                    ("  pass ", Color::Green)
                } else if check.critical {
                    ("  FAIL ", Color::Red)
                } else {
                    ("  warn ", Color::Yellow)
                };
                ListItem::new(Line::from(vec![
                    Span::styled(
                        icon,
                        Style::default().fg(color).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(&check.name, Style::default().fg(Color::White)),
                    Span::styled(" — ", Style::default().fg(Color::DarkGray)),
                    Span::styled(&check.message, Style::default().fg(Color::DarkGray)),
                ]))
            })
            .collect();

        let list = List::new(items).block(
            Block::default()
                .title(" Results ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        );
        frame.render_widget(list, chunks[1]);
    }

    // Help text
    let help_text = if app.check_results.is_empty() {
        "No workspaces configured. Run 'gisa init' then 'gisa setup' to get started."
    } else {
        "Run 'gisa setup' to configure a workspace, then restart the TUI."
    };
    let help = Paragraph::new(Line::from(Span::styled(
        help_text,
        Style::default().fg(Color::Yellow),
    )))
    .centered()
    .block(Block::default().borders(Borders::TOP));
    frame.render_widget(help, chunks[2]);

    status_bar::render(frame, chunks[3], "Enter: Check  q: Quit");
}
