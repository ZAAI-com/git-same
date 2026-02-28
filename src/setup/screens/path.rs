//! Step 4: Base path screen with folder navigation and live preview.

use crate::setup::state::SetupState;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph};
use ratatui::Frame;

pub fn render(state: &SetupState, frame: &mut Frame, area: Rect) {
    let popup_open = state.path_browse_mode;
    let list_items = if popup_open {
        0
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

    let accent = if popup_open {
        Color::DarkGray
    } else {
        Color::Cyan
    };
    let muted = Color::DarkGray;
    let input_text_color = if popup_open {
        Color::DarkGray
    } else {
        Color::White
    };

    // Title and info (above input)
    let title_lines = vec![
        Line::from(Span::styled(
            "  Where should repositories be cloned?",
            Style::default().fg(accent).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            "  Repos will be organized as: <path>/<org>/<repo>",
            Style::default().fg(muted),
        )),
        Line::from(Span::styled(
            "  Base path starts at terminal folder. Press [b] to change it.",
            Style::default().fg(muted),
        )),
    ];
    frame.render_widget(Paragraph::new(title_lines), chunks[0]);

    // Path input with styled border
    let input_style = Style::default().fg(input_text_color);

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
    let border_color = if state.path_suggestions_mode {
        Color::DarkGray
    } else {
        accent
    };
    let input = Paragraph::new(input_line).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Base Path ")
            .border_type(border_type)
            .border_style(Style::default().fg(border_color)),
    );
    frame.render_widget(input, chunks[1]);

    // Suggestions or completions list
    if state.path_suggestions_mode && !state.path_suggestions.is_empty() {
        render_suggestions(state, frame, chunks[2]);
    } else if !state.path_suggestions_mode && !state.path_completions.is_empty() {
        render_completions(state, frame, chunks[2]);
    }

    // Preview + error
    let mut preview_lines: Vec<Line> = Vec::new();
    let preview_path = &state.base_path;
    if !preview_path.is_empty() {
        preview_lines.push(Line::from(Span::styled(
            "  Preview:",
            Style::default().fg(muted),
        )));
        preview_lines.push(Line::from(Span::styled(
            format!("    {preview_path}/acme-corp/my-repo/"),
            Style::default().fg(muted),
        )));
    }

    if let Some(ref err) = state.error_message {
        preview_lines.push(Line::raw(""));
        preview_lines.push(Line::from(Span::styled(
            format!("  {}", err),
            Style::default().fg(muted),
        )));
    }

    frame.render_widget(Paragraph::new(preview_lines), chunks[3]);
    if popup_open {
        render_browse_popup(state, frame, area);
    }
}

fn render_browse_popup(state: &SetupState, frame: &mut Frame, area: Rect) {
    let popup_area = centered_area(area, 80, 80);
    frame.render_widget(Clear, popup_area);

    let popup = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Thick)
        .border_style(Style::default().fg(Color::Cyan));
    let inner = popup.inner(popup_area);
    frame.render_widget(popup, popup_area);

    let show_message = state.path_browse_error.is_some() || state.path_browse_info.is_some();
    let rows = Layout::vertical([
        Constraint::Length(3), // header
        Constraint::Length(1), // path
        Constraint::Min(3),    // tree
        Constraint::Length(if show_message { 1 } else { 0 }),
        Constraint::Length(1), // footer
    ])
    .split(inner);

    render_popup_header(frame, rows[0]);

    let path_line = Line::from(vec![
        Span::styled("Path: ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            &state.path_browse_current_dir,
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
    ]);
    frame.render_widget(Paragraph::new(path_line), rows[1]);

    render_browse_tree(state, frame, rows[2]);

    if show_message {
        let message = state
            .path_browse_error
            .as_ref()
            .map(|msg| {
                (
                    msg.as_str(),
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                )
            })
            .or_else(|| {
                state
                    .path_browse_info
                    .as_ref()
                    .map(|msg| (msg.as_str(), Style::default().fg(Color::DarkGray)))
            });
        if let Some((msg, style)) = message {
            frame.render_widget(
                Paragraph::new(Line::from(Span::styled(msg, style))),
                rows[3],
            );
        }
    }

    render_popup_footer(frame, rows[4]);
}

fn render_popup_header(frame: &mut Frame, area: Rect) {
    let header = Paragraph::new("Local Folder Navigator")
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::DarkGray)),
        );
    frame.render_widget(header, area);
}

fn render_browse_tree(state: &SetupState, frame: &mut Frame, area: Rect) {
    if area.height == 0 {
        return;
    }
    let mut lines: Vec<Line> = Vec::new();
    if state.path_browse_entries.is_empty() {
        lines.push(Line::from(Span::styled(
            "  (No folders available)",
            Style::default().fg(Color::DarkGray),
        )));
        frame.render_widget(Paragraph::new(lines), area);
        return;
    }

    let visible = area.height as usize;
    let selection = state
        .path_browse_index
        .min(state.path_browse_entries.len().saturating_sub(1));
    let half = visible / 2;
    let mut start = selection.saturating_sub(half);
    if start + visible > state.path_browse_entries.len() {
        start = state.path_browse_entries.len().saturating_sub(visible);
    }

    for (i, entry) in state
        .path_browse_entries
        .iter()
        .enumerate()
        .skip(start)
        .take(visible)
    {
        let is_selected = i == selection;
        let pointer = if is_selected { "› " } else { "  " };
        let icon = if entry.has_children {
            if entry.expanded {
                "▾ "
            } else {
                "▸ "
            }
        } else {
            "  "
        };
        let style = if is_selected {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };
        lines.push(Line::from(vec![
            Span::styled(pointer, style),
            Span::styled(
                "  ".repeat(entry.depth as usize),
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(icon, style),
            Span::styled(&entry.label, style),
        ]));
    }

    frame.render_widget(Paragraph::new(lines), area);
}

fn render_popup_footer(frame: &mut Frame, area: Rect) {
    let left = "[Esc] Close";
    let center = "[←] Parent [↑/↓] Move [→] Open";
    let right = "[Enter] Select";
    let cols = Layout::horizontal([
        Constraint::Length(left.chars().count() as u16),
        Constraint::Min(0),
        Constraint::Length(right.chars().count() as u16),
    ])
    .split(area);

    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            left,
            Style::default().fg(Color::DarkGray),
        ))),
        cols[0],
    );
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            center,
            Style::default().fg(Color::Cyan),
        )))
        .alignment(Alignment::Center),
        cols[1],
    );
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            right,
            Style::default().fg(Color::DarkGray),
        )))
        .alignment(Alignment::Right),
        cols[2],
    );
}

fn centered_area(area: Rect, width_pct: u16, height_pct: u16) -> Rect {
    let top = (100 - height_pct) / 2;
    let bottom = 100 - height_pct - top;
    let left = (100 - width_pct) / 2;
    let right = 100 - width_pct - left;

    let vertical = Layout::vertical([
        Constraint::Percentage(top),
        Constraint::Percentage(height_pct),
        Constraint::Percentage(bottom),
    ])
    .split(area);
    let horizontal = Layout::horizontal([
        Constraint::Percentage(left),
        Constraint::Percentage(width_pct),
        Constraint::Percentage(right),
    ])
    .split(vertical[1]);
    horizontal[1]
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
