//! Step 3: Base path input screen with suggestions and tab completion.

use crate::setup::state::SetupState;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

pub fn render(state: &SetupState, frame: &mut Frame, area: Rect) {
    let list_items = if state.path_suggestions_mode {
        state.path_suggestions.len()
    } else {
        state.path_completions.len()
    };
    let list_height = if list_items > 0 {
        (list_items as u16 + 1).min(7)
    } else {
        0
    };

    let chunks = Layout::vertical([
        Constraint::Length(3),          // Title
        Constraint::Length(3),          // Input
        Constraint::Length(list_height), // Suggestions or completions
        Constraint::Min(3),            // Info
        Constraint::Length(2),         // Help
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
    let input_style = if state.path_suggestions_mode {
        Style::default().fg(Color::DarkGray)
    } else {
        Style::default().fg(Color::Yellow)
    };
    let cursor_pos = state.path_cursor.min(state.base_path.len());

    let input_line = Line::from(vec![
        Span::styled("Path: ", Style::default().fg(Color::White)),
        Span::styled(&state.base_path, input_style),
    ]);
    let border_style = if state.path_suggestions_mode {
        Style::default().fg(Color::DarkGray)
    } else {
        Style::default()
    };
    let input = Paragraph::new(input_line).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Base Path")
            .border_style(border_style),
    );
    frame.render_widget(input, chunks[1]);

    // Only show cursor in input mode
    if !state.path_suggestions_mode {
        let cursor_x = chunks[1].x + 1 + 6 + cursor_pos as u16;
        let cursor_y = chunks[1].y + 1;
        frame.set_cursor_position((cursor_x, cursor_y));
    }

    // Suggestions or completions list
    if state.path_suggestions_mode && !state.path_suggestions.is_empty() {
        render_suggestions(state, frame, chunks[2]);
    } else if !state.path_suggestions_mode && !state.path_completions.is_empty() {
        render_completions(state, frame, chunks[2]);
    }

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
    frame.render_widget(info, chunks[3]);

    // Error
    if let Some(ref err) = state.error_message {
        let error = Paragraph::new(Span::styled(err.as_str(), Style::default().fg(Color::Red)));
        frame.render_widget(error, chunks[3]);
    }

    // Help (mode-dependent)
    let help_text = if state.path_suggestions_mode {
        "\u{2191}/\u{2193} Select  Enter Confirm  Type to edit  Esc Back"
    } else if !state.path_completions.is_empty() {
        "Tab Complete  Enter Confirm  Esc Back"
    } else {
        "Enter Confirm  Esc Back"
    };
    let help = Paragraph::new(help_text).style(Style::default().fg(Color::DarkGray));
    frame.render_widget(help, chunks[4]);
}

fn render_suggestions(state: &SetupState, frame: &mut Frame, area: Rect) {
    let mut lines = vec![Line::from(Span::styled(
        "  Suggestions:",
        Style::default().fg(Color::DarkGray),
    ))];

    for (i, suggestion) in state.path_suggestions.iter().enumerate() {
        let is_selected = i == state.path_suggestion_index;
        let marker = if is_selected { "  \u{25b8} " } else { "    " };
        let path_style = if is_selected {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        let mut spans = vec![
            Span::styled(marker, path_style),
            Span::styled(&suggestion.path, path_style),
        ];
        if !suggestion.label.is_empty() {
            spans.push(Span::styled(
                format!("  ({})", suggestion.label),
                Style::default().fg(Color::DarkGray),
            ));
        }
        lines.push(Line::from(spans));
    }

    frame.render_widget(Paragraph::new(lines), area);
}

fn render_completions(state: &SetupState, frame: &mut Frame, area: Rect) {
    let mut lines: Vec<Line> = Vec::new();
    for (i, path) in state.path_completions.iter().enumerate() {
        if i >= 6 {
            break;
        }
        let style = if i == state.path_completion_index {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        lines.push(Line::from(Span::styled(format!("    {path}"), style)));
    }

    frame.render_widget(Paragraph::new(lines), area);
}
