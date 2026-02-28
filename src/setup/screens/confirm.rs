//! Step 5: Review and save screen with bordered summary card.

use crate::setup::state::SetupState;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

pub fn render(state: &SetupState, frame: &mut Frame, area: Rect) {
    let chunks = Layout::vertical([
        Constraint::Length(3), // Title
        Constraint::Length(8), // Summary card
        Constraint::Min(3),    // Info + error
    ])
    .split(area);

    // Title
    let title = Paragraph::new(Line::from(Span::styled(
        "  Review Workspace Configuration",
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )));
    frame.render_widget(title, chunks[0]);

    // Summary card
    let provider = state.selected_provider();
    let selected_orgs = state.selected_orgs();
    let orgs_display = if selected_orgs.is_empty() {
        "all organizations".to_string()
    } else if selected_orgs.len() <= 3 {
        selected_orgs.join(", ")
    } else {
        format!(
            "{}, ... +{} more",
            selected_orgs[..2].join(", "),
            selected_orgs.len() - 2
        )
    };

    let label_style = Style::default().fg(Color::DarkGray);
    let value_style = Style::default()
        .fg(Color::White)
        .add_modifier(Modifier::BOLD);

    let summary_lines = vec![
        Line::from(vec![
            Span::styled("  Provider      ", label_style),
            Span::styled(provider.display_name(), value_style),
        ]),
        Line::from(vec![
            Span::styled("  Username      ", label_style),
            Span::styled(
                format!("@{}", state.username.as_deref().unwrap_or("unknown")),
                value_style,
            ),
        ]),
        Line::from(vec![
            Span::styled("  Base Path     ", label_style),
            Span::styled(&state.base_path, value_style),
        ]),
        Line::from(vec![
            Span::styled("  Organizations ", label_style),
            Span::styled(&orgs_display, value_style),
        ]),
    ];

    let summary = Paragraph::new(summary_lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    frame.render_widget(summary, chunks[1]);

    // Info + error
    let mut info_lines: Vec<Line> = Vec::new();
    info_lines.push(Line::raw(""));
    info_lines.push(Line::from(Span::styled(
        format!("  Config will be saved to: {}/.git-same/", state.base_path),
        Style::default().fg(Color::DarkGray),
    )));
    info_lines.push(Line::raw(""));
    info_lines.push(Line::from(Span::styled(
        "  Press Enter to save and continue",
        Style::default().fg(Color::Yellow),
    )));

    if let Some(ref err) = state.error_message {
        info_lines.push(Line::raw(""));
        info_lines.push(Line::from(Span::styled(
            format!("  Error: {}", err),
            Style::default().fg(Color::Red),
        )));
    }

    frame.render_widget(Paragraph::new(info_lines), chunks[2]);
}

#[cfg(test)]
#[path = "confirm_tests.rs"]
mod tests;
