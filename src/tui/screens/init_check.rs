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
                    ("  pass ", Color::Rgb(21, 128, 61))
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

    // Help text / config status
    let help_lines = if app.config_created {
        let path = app
            .config_path_display
            .as_deref()
            .unwrap_or("~/.config/git-same/config.toml");
        vec![Line::from(vec![
            Span::styled(
                "  Config created at ",
                Style::default().fg(Color::Rgb(21, 128, 61)),
            ),
            Span::styled(path, Style::default().fg(Color::Cyan)),
            Span::styled(
                "  — Press 's' to set up a workspace.",
                Style::default().fg(Color::Yellow),
            ),
        ])]
    } else if !app.check_results.is_empty() {
        vec![Line::from(vec![
            Span::styled(
                "  Press 'c' to create config",
                Style::default().fg(Color::Yellow),
            ),
            Span::styled(
                "  or 's' to set up a workspace.",
                Style::default().fg(Color::DarkGray),
            ),
        ])]
    } else {
        vec![Line::from(Span::styled(
            "  No workspaces configured. Press 's' to set up a workspace.",
            Style::default().fg(Color::Yellow),
        ))]
    };

    let help = Paragraph::new(help_lines).block(Block::default().borders(Borders::TOP));
    frame.render_widget(help, chunks[2]);

    let hint = if !app.check_results.is_empty() && !app.config_created {
        "Enter: Re-check  c: Create Config  s: Setup  qq: Quit"
    } else {
        "s: Setup  Enter: Check  qq: Quit"
    };
    status_bar::render(frame, chunks[3], hint);
}
