//! Step 4: Base path input screen with suggestions, tab completion, and live preview.

use crate::setup::state::SetupState;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};
use ratatui::Frame;

pub fn render(state: &SetupState, frame: &mut Frame, area: Rect) {
    let list_items = if state.path_browse_mode {
        state.path_browse_entries.len() + 5
    } else if state.path_suggestions_mode {
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
        Constraint::Length(4),           // Title + info
        Constraint::Length(3),           // Input
        Constraint::Length(list_height), // Suggestions or completions
        Constraint::Min(3),              // Preview + error
    ])
    .split(area);

    // Title and info (above input)
    let title_lines = vec![
        Line::from(Span::styled(
            "  Where should repositories be cloned?",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            "  Repos will be organized as: <path>/<org>/<repo>",
            Style::default().fg(Color::DarkGray),
        )),
    ];
    frame.render_widget(Paragraph::new(title_lines), chunks[0]);

    // Path input with styled border
    let input_style = if state.path_browse_mode {
        Style::default().fg(Color::Cyan)
    } else if state.path_suggestions_mode {
        Style::default().fg(Color::DarkGray)
    } else {
        Style::default().fg(Color::Yellow)
    };
    let cursor_pos = state.path_cursor.min(state.base_path.len());

    let input_line = Line::from(vec![
        Span::styled("  ", Style::default()),
        Span::styled(&state.base_path, input_style),
    ]);
    let border_type = if state.path_browse_mode {
        BorderType::Thick
    } else if state.path_suggestions_mode {
        BorderType::Plain
    } else {
        BorderType::Thick
    };
    let border_color = if state.path_browse_mode {
        Color::Cyan
    } else if state.path_suggestions_mode {
        Color::DarkGray
    } else {
        Color::Cyan
    };
    let input = Paragraph::new(input_line).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Base Path ")
            .border_type(border_type)
            .border_style(Style::default().fg(border_color)),
    );
    frame.render_widget(input, chunks[1]);

    // Show cursor in input mode
    if !state.path_suggestions_mode && !state.path_browse_mode {
        let cursor_x = chunks[1].x + 1 + 2 + cursor_pos as u16;
        let cursor_y = chunks[1].y + 1;
        frame.set_cursor_position((cursor_x, cursor_y));
    }

    // Suggestions or completions list
    if state.path_browse_mode {
        render_browse(state, frame, chunks[2]);
    } else if state.path_suggestions_mode && !state.path_suggestions.is_empty() {
        render_suggestions(state, frame, chunks[2]);
    } else if !state.path_suggestions_mode && !state.path_completions.is_empty() {
        render_completions(state, frame, chunks[2]);
    }

    // Preview + error
    let mut preview_lines: Vec<Line> = Vec::new();
    let preview_path = if state.path_browse_mode {
        &state.path_browse_current_dir
    } else {
        &state.base_path
    };
    if !preview_path.is_empty() {
        preview_lines.push(Line::from(Span::styled(
            "  Preview:",
            Style::default().fg(Color::DarkGray),
        )));
        preview_lines.push(Line::from(Span::styled(
            format!("    {preview_path}/acme-corp/my-repo/"),
            Style::default().fg(Color::DarkGray),
        )));
    }

    if let Some(ref err) = state.error_message {
        preview_lines.push(Line::raw(""));
        preview_lines.push(Line::from(Span::styled(
            format!("  {}", err),
            Style::default().fg(Color::Red),
        )));
    }

    frame.render_widget(Paragraph::new(preview_lines), chunks[3]);
}

fn render_browse(state: &SetupState, frame: &mut Frame, area: Rect) {
    let hidden_state = if state.path_browse_show_hidden {
        "on"
    } else {
        "off"
    };

    let mut lines: Vec<Line> = vec![
        Line::from(Span::styled(
            "  Folder Navigator:",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(Span::styled(
            format!("    {}", state.path_browse_current_dir),
            Style::default().fg(Color::Cyan),
        )),
        Line::from(Span::styled(
            format!("    Hidden folders: {hidden_state}  (press . to toggle)"),
            Style::default().fg(Color::DarkGray),
        )),
    ];

    if let Some(ref info) = state.path_browse_info {
        lines.push(Line::from(Span::styled(
            format!("    {}", info),
            Style::default().fg(Color::DarkGray),
        )));
    }

    if let Some(ref err) = state.path_browse_error {
        lines.push(Line::from(Span::styled(
            format!("    {}", err),
            Style::default().fg(Color::Red),
        )));
    }

    if state.path_browse_entries.is_empty() {
        lines.push(Line::from(Span::styled(
            "    (No folders available)",
            Style::default().fg(Color::DarkGray),
        )));
    } else {
        let visible = area.height.saturating_sub(lines.len() as u16) as usize;
        let start = state
            .path_browse_index
            .saturating_sub(visible.saturating_sub(1));
        for (i, entry) in state
            .path_browse_entries
            .iter()
            .enumerate()
            .skip(start)
            .take(visible)
        {
            let is_selected = i == state.path_browse_index;
            let marker = if is_selected { "  > " } else { "    " };
            let style = if is_selected {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            lines.push(Line::from(vec![
                Span::styled(marker, style),
                Span::styled(&entry.label, style),
            ]));
        }
    }

    frame.render_widget(Paragraph::new(lines), area);
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
                .fg(Color::Cyan)
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

#[cfg(test)]
#[path = "path_tests.rs"]
mod tests;
