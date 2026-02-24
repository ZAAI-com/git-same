//! Workspace selector screen — pick which workspace to use.

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
        Constraint::Min(6),    // Workspace list
        Constraint::Length(1), // Status bar
    ])
    .split(frame.area());

    // Title
    let title = Paragraph::new(Line::from(Span::styled(
        " Select Workspace ",
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

    // Workspace list
    let items: Vec<ListItem> = app
        .workspaces
        .iter()
        .enumerate()
        .map(|(i, ws)| {
            let marker = if i == app.workspace_index { ">" } else { " " };
            let style = if i == app.workspace_index {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            let is_default = app.config.default_workspace.as_deref() == Some(ws.name.as_str());
            let last_synced = ws.last_synced.as_deref().unwrap_or("never synced");
            let org_info = if ws.orgs.is_empty() {
                "all orgs".to_string()
            } else {
                format!("{} orgs", ws.orgs.len())
            };

            let mut spans = vec![
                Span::styled(format!("  {} ", marker), style),
                Span::styled(format!("{:<16}", ws.name), style),
            ];
            if is_default {
                spans.push(Span::styled(
                    "(default) ",
                    Style::default().fg(Color::Green),
                ));
            }
            spans.extend([
                Span::styled(&ws.base_path, Style::default().fg(Color::DarkGray)),
                Span::styled("  (", Style::default().fg(Color::DarkGray)),
                Span::styled(org_info, Style::default().fg(Color::DarkGray)),
                Span::styled(", ", Style::default().fg(Color::DarkGray)),
                Span::styled(last_synced, Style::default().fg(Color::DarkGray)),
                Span::styled(")", Style::default().fg(Color::DarkGray)),
            ]);

            ListItem::new(Line::from(spans))
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .title(" Workspaces ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    frame.render_widget(list, chunks[1]);

    status_bar::render(
        frame,
        chunks[2],
        "j/k: Navigate  Enter: Select  d: Set default  q: Quit",
    );
}
