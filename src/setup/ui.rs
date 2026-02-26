//! Setup wizard render dispatcher.

use super::screens;
use super::state::{SetupState, SetupStep};
use crate::banner;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

/// Render the setup wizard.
pub fn render(state: &SetupState, frame: &mut Frame) {
    let area = frame.area();
    let height = area.height;

    // Graceful degradation for small terminals
    let show_banner = height >= 30;
    let show_progress = height >= 20;

    let mut constraints = Vec::new();
    if show_banner {
        constraints.push(Constraint::Length(6)); // Banner
    }
    constraints.push(Constraint::Length(2)); // Title
    if show_progress {
        constraints.push(Constraint::Length(3)); // Step progress indicator
    }
    constraints.push(Constraint::Min(8)); // Step content
    constraints.push(Constraint::Length(2)); // Status bar

    let chunks = Layout::vertical(constraints).split(area);

    let mut idx = 0;

    // Banner
    if show_banner {
        if state.step == SetupStep::Complete {
            let phase = (state.tick_count % 100) as f64 / 100.0;
            banner::render_animated_banner(frame, chunks[idx], phase);
        } else {
            banner::render_banner(frame, chunks[idx]);
        }
        idx += 1;
    }

    // Title
    let title_text = if state.step == SetupStep::Welcome {
        ""
    } else if state.is_first_setup {
        "Workspace Setup"
    } else {
        "New Workspace"
    };
    if !title_text.is_empty() {
        let title = Paragraph::new(title_text)
            .style(
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Center);
        frame.render_widget(title, chunks[idx]);
    }
    idx += 1;

    // Step progress indicator
    if show_progress {
        render_step_progress(state, frame, chunks[idx]);
        idx += 1;
    }

    // Step content
    let content_area = chunks[idx];
    idx += 1;

    match state.step {
        SetupStep::Welcome => screens::welcome::render(state, frame, content_area),
        SetupStep::SelectProvider => screens::provider::render(state, frame, content_area),
        SetupStep::Authenticate => screens::auth::render(state, frame, content_area),
        SetupStep::SelectOrgs => screens::orgs::render(state, frame, content_area),
        SetupStep::SelectPath => screens::path::render(state, frame, content_area),
        SetupStep::Confirm => screens::confirm::render(state, frame, content_area),
        SetupStep::Complete => screens::complete::render(state, frame, content_area),
    }

    // Status bar
    render_status_bar(state, frame, chunks[idx]);
}

/// Render the step progress indicator with nodes and connectors.
fn render_step_progress(state: &SetupState, frame: &mut Frame, area: Rect) {
    let steps = ["Provider", "Auth", "Orgs", "Path", "Save"];
    let current = state.step_number(); // 0 for Welcome, 1-5 for steps, 5 for Complete

    let green = Style::default().fg(Color::Rgb(21, 128, 61));
    let green_bold = green.add_modifier(Modifier::BOLD);
    let cyan_bold = Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD);
    let dim = Style::default().fg(Color::DarkGray);

    let segments = Layout::horizontal([
        Constraint::Ratio(3, 23),
        Constraint::Ratio(2, 23),
        Constraint::Ratio(3, 23),
        Constraint::Ratio(2, 23),
        Constraint::Ratio(3, 23),
        Constraint::Ratio(2, 23),
        Constraint::Ratio(3, 23),
        Constraint::Ratio(2, 23),
        Constraint::Ratio(3, 23),
    ])
    .split(area);

    let mut node_spans: Vec<Span> = Vec::new();
    let mut label_spans: Vec<Span> = Vec::new();

    for (i, label) in steps.iter().enumerate() {
        let step_num = i + 1;
        let node_width = segments[i * 2].width as usize;
        let node_style = if step_num < current || state.step == SetupStep::Complete {
            green_bold
        } else if step_num == current {
            cyan_bold
        } else {
            dim
        };
        let label_style = if step_num < current || state.step == SetupStep::Complete {
            green
        } else if step_num == current {
            cyan_bold
        } else {
            dim
        };

        let node_text = if step_num < current || state.step == SetupStep::Complete {
            "(\u{2713})".to_string()
        } else {
            format!("({})", step_num)
        };

        node_spans.push(Span::styled(
            center_cell(&node_text, node_width),
            node_style,
        ));
        label_spans.push(Span::styled(center_cell(label, node_width), label_style));

        if i < steps.len() - 1 {
            let connector_width = segments[i * 2 + 1].width as usize;
            let connector_done = step_num < current || state.step == SetupStep::Complete;
            let connector_style = if connector_done { green } else { dim };
            node_spans.push(Span::styled(
                connector_cell(connector_width, connector_done),
                connector_style,
            ));
            label_spans.push(Span::raw(" ".repeat(connector_width)));
        }
    }

    let widget = Paragraph::new(vec![Line::from(node_spans), Line::from(label_spans)]);
    frame.render_widget(widget, area);
}

fn center_cell(text: &str, width: usize) -> String {
    if width == 0 {
        return String::new();
    }

    let text = if text.chars().count() > width {
        text.chars().take(width).collect::<String>()
    } else {
        text.to_string()
    };
    let text_width = text.chars().count();
    let left_pad = (width - text_width) / 2;
    let right_pad = width - text_width - left_pad;
    format!("{}{}{}", " ".repeat(left_pad), text, " ".repeat(right_pad))
}

fn connector_cell(width: usize, completed: bool) -> String {
    if width == 0 {
        return String::new();
    }

    if completed {
        return "\u{2501}".repeat(width);
    }

    // Dashed connector for upcoming steps.
    let mut out = String::with_capacity(width);
    for i in 0..width {
        if i % 2 == 0 {
            out.push('\u{2500}');
        } else {
            out.push(' ');
        }
    }
    out
}

/// Render the 2-line status bar with actions and navigation hints.
fn render_status_bar(state: &SetupState, frame: &mut Frame, area: Rect) {
    let blue = Style::default()
        .fg(Color::Rgb(37, 99, 235))
        .add_modifier(Modifier::BOLD);
    let dim = Style::default().fg(Color::DarkGray);

    let (actions, nav) = match state.step {
        SetupStep::Welcome => (
            vec![
                Span::styled(" [Enter]", blue),
                Span::styled(" Get Started", dim),
            ],
            vec![
                Span::styled(" [Esc]", blue),
                Span::styled(" Cancel  ", dim),
                Span::styled("[q]", blue),
                Span::styled(" Quit", dim),
            ],
        ),
        SetupStep::SelectProvider => (
            vec![Span::styled(" [Enter]", blue), Span::styled(" Select", dim)],
            vec![
                Span::styled(" [←] [↑] [↓] [→]", blue),
                Span::styled(" Move  ", dim),
                Span::styled("[Esc]", blue),
                Span::styled(" Cancel  ", dim),
                Span::styled("[q]", blue),
                Span::styled(" Quit", dim),
            ],
        ),
        SetupStep::Authenticate => {
            use super::state::AuthStatus;
            let action_label = match &state.auth_status {
                AuthStatus::Pending | AuthStatus::Failed(_) => " Authenticate",
                AuthStatus::Success => " Continue",
                AuthStatus::Checking => " Checking...",
            };
            (
                vec![
                    Span::styled(" [Enter]", blue),
                    Span::styled(action_label, dim),
                ],
                vec![
                    Span::styled(" [Esc]", blue),
                    Span::styled(" Back  ", dim),
                    Span::styled("[q]", blue),
                    Span::styled(" Quit", dim),
                ],
            )
        }
        SetupStep::SelectPath => {
            if state.path_browse_mode {
                (
                    vec![
                        Span::styled(" [Enter]", blue),
                        Span::styled(" Use Folder  ", dim),
                        Span::styled("[\u{2190}] [\u{2192}]", blue),
                        Span::styled(" Parent/Open", dim),
                    ],
                    vec![
                        Span::styled(" [\u{2191}] [\u{2193}]", blue),
                        Span::styled(" Move  ", dim),
                        Span::styled("[Esc]", blue),
                        Span::styled(" Close  ", dim),
                        Span::styled("[q]", blue),
                        Span::styled(" Quit", dim),
                    ],
                )
            } else if state.path_suggestions_mode {
                (
                    vec![
                        Span::styled(" [Enter]", blue),
                        Span::styled(" Confirm  ", dim),
                        Span::styled("[Tab]", blue),
                        Span::styled(" Edit  ", dim),
                        Span::styled("[b]", blue),
                        Span::styled(" Browse", dim),
                    ],
                    vec![
                        Span::styled(" [←] [↑] [↓] [→]", blue),
                        Span::styled(" Move  ", dim),
                        Span::styled("[Esc]", blue),
                        Span::styled(" Back  ", dim),
                        Span::styled("[q]", blue),
                        Span::styled(" Quit", dim),
                    ],
                )
            } else {
                (
                    vec![
                        Span::styled(" [Enter]", blue),
                        Span::styled(" Confirm  ", dim),
                        Span::styled("[Tab]", blue),
                        Span::styled(" Complete  ", dim),
                        Span::styled("[Ctrl+b]", blue),
                        Span::styled(" Browse", dim),
                    ],
                    vec![
                        Span::styled(" [Esc]", blue),
                        Span::styled(" Back  ", dim),
                        Span::styled("[q]", blue),
                        Span::styled(" Quit", dim),
                    ],
                )
            }
        }
        SetupStep::SelectOrgs => {
            if state.org_loading {
                (
                    vec![Span::styled(" Discovering organizations...", dim)],
                    vec![
                        Span::styled(" [Esc]", blue),
                        Span::styled(" Back  ", dim),
                        Span::styled("[q]", blue),
                        Span::styled(" Quit", dim),
                    ],
                )
            } else {
                (
                    vec![
                        Span::styled(" [Space]", blue),
                        Span::styled(" Toggle  ", dim),
                        Span::styled("[a]", blue),
                        Span::styled(" All  ", dim),
                        Span::styled("[n]", blue),
                        Span::styled(" None  ", dim),
                        Span::styled("[Enter]", blue),
                        Span::styled(" Confirm", dim),
                    ],
                    vec![
                        Span::styled(" [←] [↑] [↓] [→]", blue),
                        Span::styled(" Move  ", dim),
                        Span::styled("[Esc]", blue),
                        Span::styled(" Back  ", dim),
                        Span::styled("[q]", blue),
                        Span::styled(" Quit", dim),
                    ],
                )
            }
        }
        SetupStep::Confirm => (
            vec![Span::styled(" [Enter]", blue), Span::styled(" Save", dim)],
            vec![
                Span::styled(" [Esc]", blue),
                Span::styled(" Back  ", dim),
                Span::styled("[q]", blue),
                Span::styled(" Quit", dim),
            ],
        ),
        SetupStep::Complete => (
            vec![
                Span::styled(" [Enter]", blue),
                Span::styled(" Dashboard  ", dim),
                Span::styled("[s]", blue),
                Span::styled(" Sync Now", dim),
            ],
            vec![
                Span::styled(" [Esc]", blue),
                Span::styled(" Back  ", dim),
                Span::styled("[q]", blue),
                Span::styled(" Quit", dim),
            ],
        ),
    };

    let rows = Layout::vertical([Constraint::Length(1), Constraint::Length(1)]).split(area);
    let step_num = state.step_number();
    let step_text = if step_num > 0 {
        Some(format!("Step {} of {}", step_num, SetupState::TOTAL_STEPS))
    } else {
        None
    };
    let step_width = step_text
        .as_ref()
        .map(|s| s.chars().count() as u16 + 1)
        .unwrap_or(0);
    let top_cols =
        Layout::horizontal([Constraint::Min(0), Constraint::Length(step_width)]).split(rows[0]);

    frame.render_widget(Paragraph::new(Line::from(actions)), top_cols[0]);
    if let Some(text) = step_text {
        let step_widget = Paragraph::new(Line::from(Span::styled(text, dim))).right_aligned();
        frame.render_widget(step_widget, top_cols[1]);
    }

    frame.render_widget(Paragraph::new(Line::from(nav)), rows[1]);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn center_cell_matches_width() {
        let out = center_cell("Auth", 10);
        assert_eq!(out.chars().count(), 10);
        assert!(out.contains("Auth"));
    }

    #[test]
    fn connector_cell_matches_width() {
        assert_eq!(connector_cell(7, true).chars().count(), 7);
        assert_eq!(connector_cell(7, false).chars().count(), 7);
    }
}
