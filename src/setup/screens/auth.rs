//! Step 2: Authentication screen.

use crate::setup::state::{AuthStatus, SetupState};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

pub fn render(state: &SetupState, frame: &mut Frame, area: Rect) {
    let chunks = Layout::vertical([
        Constraint::Length(3), // Title
        Constraint::Min(6),    // Status
        Constraint::Length(2), // Help
    ])
    .split(area);

    // Title
    let provider = state.selected_provider();
    let title = Paragraph::new(format!("Authenticate with {}", provider.display_name()))
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .block(Block::default().borders(Borders::BOTTOM));
    frame.render_widget(title, chunks[0]);

    // Auth status
    let lines: Vec<Line> = match &state.auth_status {
        AuthStatus::Pending => vec![Line::from(Span::styled(
            "Press Enter to authenticate...",
            Style::default().fg(Color::Yellow),
        ))],
        AuthStatus::Checking => vec![Line::from(Span::styled(
            "⏳ Authenticating...",
            Style::default().fg(Color::Yellow),
        ))],
        AuthStatus::Success => {
            let mut lines = vec![Line::from(Span::styled(
                "✓ Authenticated",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ))];
            if let Some(ref username) = state.username {
                lines.push(Line::from(vec![
                    Span::raw("  Logged in as: "),
                    Span::styled(
                        username.as_str(),
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]));
            }
            lines.push(Line::raw(""));
            lines.push(Line::from(Span::styled(
                "Press Enter to continue",
                Style::default().fg(Color::DarkGray),
            )));
            lines
        }
        AuthStatus::Failed(msg) => vec![
            Line::from(Span::styled(
                "✗ Authentication failed",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            )),
            Line::raw(""),
            Line::from(Span::styled(msg.as_str(), Style::default().fg(Color::Red))),
            Line::raw(""),
            Line::from(Span::styled(
                "Press Enter to retry, Esc to go back",
                Style::default().fg(Color::DarkGray),
            )),
        ],
    };

    let status = Paragraph::new(lines).block(Block::default().borders(Borders::NONE));
    frame.render_widget(status, chunks[1]);

    // Help
    let help =
        Paragraph::new("Enter Continue  Esc Back").style(Style::default().fg(Color::DarkGray));
    frame.render_widget(help, chunks[2]);
}
