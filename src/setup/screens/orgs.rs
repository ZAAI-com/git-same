//! Step 4: Organization selection screen.

use crate::setup::state::SetupState;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use ratatui::Frame;

pub fn render(state: &SetupState, frame: &mut Frame, area: Rect) {
    let chunks = Layout::vertical([
        Constraint::Length(3), // Title
        Constraint::Min(8),    // Org list
        Constraint::Length(2), // Help
    ])
    .split(area);

    // Title
    let selected_count = state.orgs.iter().filter(|o| o.selected).count();
    let title_text = format!(
        "Select organizations ({} of {} selected)",
        selected_count,
        state.orgs.len()
    );
    let title = Paragraph::new(title_text)
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .block(Block::default().borders(Borders::BOTTOM));
    frame.render_widget(title, chunks[0]);

    // Content
    if state.org_loading {
        let loading = Paragraph::new(Line::from(Span::styled(
            "⏳ Discovering organizations...",
            Style::default().fg(Color::Yellow),
        )));
        frame.render_widget(loading, chunks[1]);
    } else if let Some(ref err) = state.org_error {
        let error_lines = vec![
            Line::from(Span::styled(
                "Failed to discover organizations",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            )),
            Line::raw(""),
            Line::from(Span::styled(err.as_str(), Style::default().fg(Color::Red))),
            Line::raw(""),
            Line::from(Span::styled(
                "Press Enter to retry, Esc to go back",
                Style::default().fg(Color::DarkGray),
            )),
        ];
        let error = Paragraph::new(error_lines);
        frame.render_widget(error, chunks[1]);
    } else if state.orgs.is_empty() {
        let empty = Paragraph::new(Line::from(Span::styled(
            "No organizations found. Press Enter to continue (personal repos will be synced).",
            Style::default().fg(Color::DarkGray),
        )));
        frame.render_widget(empty, chunks[1]);
    } else {
        let items: Vec<ListItem> = state
            .orgs
            .iter()
            .enumerate()
            .map(|(i, org)| {
                let marker = if i == state.org_index { "▸" } else { " " };
                let checkbox = if org.selected { "[x]" } else { "[ ]" };

                let style = if i == state.org_index {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else if org.selected {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default().fg(Color::White)
                };

                ListItem::new(Line::from(vec![
                    Span::styled(format!("{} {} ", marker, checkbox), style),
                    Span::styled(&org.name, style),
                    Span::styled(
                        format!(" ({} repos)", org.repo_count),
                        Style::default().fg(Color::DarkGray),
                    ),
                ]))
            })
            .collect();

        let list = List::new(items).block(Block::default().borders(Borders::NONE));
        frame.render_widget(list, chunks[1]);
    }

    // Help
    let help_text = if state.orgs.is_empty() || state.org_loading {
        "Enter Continue  Esc Back"
    } else {
        "↑/↓ Navigate  Space Toggle  a Select All  n Deselect All  Enter Confirm  Esc Back"
    };
    let help = Paragraph::new(help_text).style(Style::default().fg(Color::DarkGray));
    frame.render_widget(help, chunks[2]);
}
