//! Step 2: Authentication screen with spinner and centered layout.

use crate::setup::state::{AuthStatus, SetupState};
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};
use ratatui::Frame;

/// Braille spinner frames.
const SPINNER: [char; 10] = [
    '\u{280b}', '\u{2819}', '\u{2839}', '\u{2838}', '\u{283c}', '\u{2834}', '\u{2826}', '\u{2827}',
    '\u{2807}', '\u{280f}',
];

pub fn render(state: &SetupState, frame: &mut Frame, area: Rect) {
    let provider = state.selected_provider();
    let green = Style::default().fg(Color::Rgb(21, 128, 61));
    let green_bold = green.add_modifier(Modifier::BOLD);

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::raw(""));

    // Title
    lines.push(Line::from(Span::styled(
        format!("Authenticate with {}", provider.display_name()),
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::raw(""));
    lines.push(Line::from(Span::styled(
        "Detection method: GitHub CLI (gh)",
        Style::default().fg(Color::DarkGray),
    )));
    lines.push(Line::raw(""));

    match &state.auth_status {
        AuthStatus::Pending => {
            lines.push(Line::from(Span::styled(
                "Press Enter to authenticate...",
                Style::default().fg(Color::Yellow),
            )));
        }
        AuthStatus::Checking => {
            let spinner_char = SPINNER[(state.tick_count as usize) % SPINNER.len()];
            lines.push(Line::from(Span::styled(
                format!("{} Authenticating...", spinner_char),
                Style::default().fg(Color::Yellow),
            )));
        }
        AuthStatus::Success => {
            lines.push(Line::from(Span::styled(
                "\u{2713} Authenticated",
                green_bold,
            )));
            if let Some(ref username) = state.username {
                lines.push(Line::from(vec![
                    Span::styled("Logged in as: ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        format!("@{}", username),
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
        }
        AuthStatus::Failed(msg) => {
            lines.push(Line::from(Span::styled(
                "\u{2717} Authentication failed",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::raw(""));
            lines.push(Line::from(Span::styled(
                msg.as_str(),
                Style::default().fg(Color::White),
            )));
            lines.push(Line::raw(""));
            lines.push(Line::from(Span::styled(
                "Ensure gh is installed and run: gh auth login",
                Style::default().fg(Color::DarkGray),
            )));
        }
    }

    let content = Paragraph::new(lines).alignment(Alignment::Center);

    // Error block styling for failed state
    let block = if matches!(state.auth_status, AuthStatus::Failed(_)) {
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Plain)
            .border_style(Style::default().fg(Color::Red))
            .title(" Error ")
            .title_style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))
    } else {
        Block::default()
    };

    frame.render_widget(content.block(block), area);
}
