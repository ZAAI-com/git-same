//! Step 1: Provider selection screen.

use crate::setup::state::SetupState;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use ratatui::Frame;

pub fn render(state: &SetupState, frame: &mut Frame, area: Rect) {
    let chunks = Layout::vertical([
        Constraint::Length(3), // Title
        Constraint::Min(8),    // Provider list
        Constraint::Length(2), // Help
    ])
    .split(area);

    // Title
    let title = Paragraph::new("Select a provider")
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .block(Block::default().borders(Borders::BOTTOM));
    frame.render_widget(title, chunks[0]);

    // Provider list
    let items: Vec<ListItem> = state
        .provider_choices
        .iter()
        .enumerate()
        .map(|(i, choice)| {
            let marker = if i == state.provider_index {
                "▸ "
            } else {
                "  "
            };

            let style = if !choice.available {
                Style::default().fg(Color::DarkGray)
            } else if i == state.provider_index {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            ListItem::new(Line::from(vec![
                Span::styled(marker, style),
                Span::styled(&choice.label, style),
            ]))
        })
        .collect();

    let list = List::new(items).block(Block::default().borders(Borders::NONE));
    frame.render_widget(list, chunks[1]);

    // Help
    let help = Paragraph::new("↑/↓ Navigate  Enter Select  Esc Cancel")
        .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(help, chunks[2]);
}
