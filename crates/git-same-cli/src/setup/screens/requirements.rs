//! Step 1: System requirements check.

use crate::setup::state::SetupState;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

pub fn render(state: &SetupState, frame: &mut Frame, area: Rect) {
    let chunks = Layout::vertical([
        Constraint::Length(2), // Title
        Constraint::Min(8),    // Check results or spinner
        Constraint::Length(3), // Config status + action hint
    ])
    .split(area);

    // Title
    let title_text = if state.is_first_setup {
        "Welcome to Git-Same"
    } else {
        "System Requirements"
    };
    let title = Paragraph::new(title_text).style(
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    );
    frame.render_widget(title, chunks[0]);

    // Check list or spinner
    if state.checks_loading {
        let spinner_frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
        let frame_idx = (state.tick_count as usize / 2) % spinner_frames.len();
        let spinner = spinner_frames[frame_idx];
        let loading = Paragraph::new(Line::from(vec![
            Span::styled(
                format!("  {} ", spinner),
                Style::default().fg(Color::Yellow),
            ),
            Span::styled(
                "Checking requirements...",
                Style::default().fg(Color::DarkGray),
            ),
        ]));
        frame.render_widget(loading, chunks[1]);
    } else if state.check_results.is_empty() {
        let placeholder = Paragraph::new(Line::from(Span::styled(
            "  Preparing checks...",
            Style::default().fg(Color::DarkGray),
        )));
        frame.render_widget(placeholder, chunks[1]);
    } else {
        let lines: Vec<Line> = state
            .check_results
            .iter()
            .map(|check| {
                let (icon, icon_color) = if check.passed {
                    ("  ✓ ", Color::Rgb(21, 128, 61))
                } else if check.critical {
                    ("  ✗ ", Color::Red)
                } else {
                    ("  ! ", Color::Yellow)
                };
                let msg_color = if check.passed {
                    Color::DarkGray
                } else if check.critical {
                    Color::Red
                } else {
                    Color::Yellow
                };
                Line::from(vec![
                    Span::styled(
                        icon,
                        Style::default().fg(icon_color).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!("{:<18}", &check.name),
                        Style::default().fg(Color::White),
                    ),
                    Span::styled(" — ", Style::default().fg(Color::DarkGray)),
                    Span::styled(&check.message, Style::default().fg(msg_color)),
                ])
            })
            .collect();
        frame.render_widget(Paragraph::new(lines), chunks[1]);
    }

    // Config status + action hint
    let mut status_lines: Vec<Line> = Vec::new();

    if let Some(ref path) = state.config_path_display {
        let (label, color) = if state.config_was_created {
            ("  Config created at ", Color::Rgb(21, 128, 61))
        } else {
            ("  Config found at ", Color::DarkGray)
        };
        status_lines.push(Line::from(vec![
            Span::styled(label, Style::default().fg(color)),
            Span::styled(path, Style::default().fg(Color::Cyan)),
        ]));
    }

    if !state.check_results.is_empty() && !state.checks_loading {
        let has_critical_fail = state.check_results.iter().any(|r| r.critical && !r.passed);
        if has_critical_fail {
            status_lines.push(Line::from(Span::styled(
                "  Fix critical requirements above to continue.",
                Style::default().fg(Color::Red),
            )));
        } else {
            status_lines.push(Line::from(vec![
                Span::styled(
                    "  All requirements met. Press ",
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(
                    "[Enter]",
                    Style::default()
                        .fg(Color::Rgb(37, 99, 235))
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" to continue.", Style::default().fg(Color::DarkGray)),
            ]));
        }
    }

    frame.render_widget(Paragraph::new(status_lines), chunks[2]);
}

#[cfg(test)]
#[path = "requirements_tests.rs"]
mod tests;
