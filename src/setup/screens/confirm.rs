//! Step 5: Review and save screen.

use crate::setup::state::SetupState;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

pub fn render(state: &SetupState, frame: &mut Frame, area: Rect) {
    let chunks = Layout::vertical([
        Constraint::Length(3), // Title
        Constraint::Min(12),   // Summary
        Constraint::Length(2), // Help
    ])
    .split(area);

    // Title
    let title = Paragraph::new("Review workspace configuration")
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .block(Block::default().borders(Borders::BOTTOM));
    frame.render_widget(title, chunks[0]);

    // Summary
    let provider = state.selected_provider();
    let selected_orgs = state.selected_orgs();
    let orgs_display = if selected_orgs.is_empty() {
        "all organizations".to_string()
    } else {
        selected_orgs.join(", ")
    };

    let label_style = Style::default().fg(Color::DarkGray);
    let value_style = Style::default()
        .fg(Color::White)
        .add_modifier(Modifier::BOLD);

    let lines = vec![
        Line::raw(""),
        Line::from(vec![
            Span::styled("  Provider:   ", label_style),
            Span::styled(provider.display_name(), value_style),
        ]),
        Line::from(vec![
            Span::styled("  Username:   ", label_style),
            Span::styled(state.username.as_deref().unwrap_or("unknown"), value_style),
        ]),
        Line::from(vec![
            Span::styled("  Base Path:  ", label_style),
            Span::styled(&state.base_path, value_style),
        ]),
        Line::from(vec![
            Span::styled("  Orgs:       ", label_style),
            Span::styled(&orgs_display, value_style),
        ]),
        Line::raw(""),
        Line::from(Span::styled(
            "  Press Enter to save, Esc to go back",
            Style::default().fg(Color::Yellow),
        )),
    ];

    // Error message
    let mut all_lines = lines;
    if let Some(ref err) = state.error_message {
        all_lines.push(Line::raw(""));
        all_lines.push(Line::from(Span::styled(
            format!("  Error: {}", err),
            Style::default().fg(Color::Red),
        )));
    }

    let summary = Paragraph::new(all_lines).block(Block::default().borders(Borders::NONE));
    frame.render_widget(summary, chunks[1]);

    // Help
    let help = Paragraph::new("Enter Save  Esc Back").style(Style::default().fg(Color::DarkGray));
    frame.render_widget(help, chunks[2]);
}
