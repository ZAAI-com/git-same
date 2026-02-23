//! Step 3: Base path input screen.

use crate::setup::state::SetupState;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

pub fn render(state: &SetupState, frame: &mut Frame, area: Rect) {
    let chunks = Layout::vertical([
        Constraint::Length(3), // Title
        Constraint::Length(3), // Input
        Constraint::Min(4),    // Info
        Constraint::Length(2), // Help
    ])
    .split(area);

    // Title
    let title = Paragraph::new("Where should repos be cloned?")
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .block(Block::default().borders(Borders::BOTTOM));
    frame.render_widget(title, chunks[0]);

    // Path input
    let input_style = Style::default().fg(Color::Yellow);
    let cursor_pos = state.path_cursor.min(state.base_path.len());

    let input_line = Line::from(vec![
        Span::styled("Path: ", Style::default().fg(Color::White)),
        Span::styled(&state.base_path, input_style),
    ]);
    let input =
        Paragraph::new(input_line).block(Block::default().borders(Borders::ALL).title("Base Path"));
    frame.render_widget(input, chunks[1]);

    // Set cursor position
    // "Path: " is 6 chars, plus border is 1 char
    let cursor_x = chunks[1].x + 1 + 6 + cursor_pos as u16;
    let cursor_y = chunks[1].y + 1;
    frame.set_cursor_position((cursor_x, cursor_y));

    // Info
    let info_lines = vec![
        Line::raw(""),
        Line::from(Span::styled(
            "This is the root directory where all repositories will be cloned.",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(Span::styled(
            "Repos will be organized as: <path>/<org>/<repo>",
            Style::default().fg(Color::DarkGray),
        )),
    ];
    let info = Paragraph::new(info_lines);
    frame.render_widget(info, chunks[2]);

    // Error
    if let Some(ref err) = state.error_message {
        let error = Paragraph::new(Span::styled(err.as_str(), Style::default().fg(Color::Red)));
        frame.render_widget(error, chunks[2]);
    }

    // Help
    let help =
        Paragraph::new("Enter Confirm  Esc Back").style(Style::default().fg(Color::DarkGray));
    frame.render_widget(help, chunks[3]);
}
