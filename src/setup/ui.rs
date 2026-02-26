//! Setup wizard render dispatcher.

use super::screens;
use super::state::{SetupState, SetupStep};
use crate::banner;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

/// Render the setup wizard.
pub fn render(state: &SetupState, frame: &mut Frame) {
    let area = frame.area();
    let height = area.height;
    let path_popup_active = state.step == SetupStep::SelectPath && state.path_browse_mode;

    // Graceful degradation for small terminals
    let show_banner = height >= 30;
    let show_progress = height >= 20;

    let mut constraints = Vec::new();
    if show_banner {
        constraints.push(Constraint::Length(6)); // Banner
    }
    constraints.push(Constraint::Length(2)); // Title
    if show_progress {
        constraints.push(Constraint::Length(4)); // Step progress indicator (with border)
    }
    constraints.push(Constraint::Min(10)); // Step content (with border)
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
                    .fg(if path_popup_active {
                        Color::DarkGray
                    } else {
                        Color::White
                    })
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Center);
        frame.render_widget(title, chunks[idx]);
    }
    idx += 1;

    // Step progress indicator
    if show_progress {
        let progress_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray));
        let progress_inner = progress_block.inner(chunks[idx]);
        frame.render_widget(progress_block, chunks[idx]);
        render_step_progress(state, frame, progress_inner, path_popup_active);
        idx += 1;
    }

    // Step content
    let content_area = chunks[idx];
    let content_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));
    let content_inner = content_block.inner(content_area);
    frame.render_widget(content_block, content_area);
    idx += 1;

    match state.step {
        SetupStep::Welcome => screens::welcome::render(state, frame, content_inner),
        SetupStep::SelectProvider => screens::provider::render(state, frame, content_inner),
        SetupStep::Authenticate => screens::auth::render(state, frame, content_inner),
        SetupStep::SelectOrgs => screens::orgs::render(state, frame, content_inner),
        SetupStep::SelectPath => screens::path::render(state, frame, content_inner),
        SetupStep::Confirm => screens::confirm::render(state, frame, content_inner),
        SetupStep::Complete => screens::complete::render(state, frame, content_inner),
    }

    // Status bar
    render_status_bar(state, frame, chunks[idx]);
}

/// Render the step progress indicator with nodes and connectors.
fn render_step_progress(state: &SetupState, frame: &mut Frame, area: Rect, dimmed: bool) {
    let steps = ["Provider", "Auth", "Orgs", "Path", "Save"];
    let current = state.step_number(); // 0 for Welcome, 1-5 for steps, 5 for Complete

    let green = if dimmed {
        Style::default().fg(Color::DarkGray)
    } else {
        Style::default().fg(Color::Rgb(21, 128, 61))
    };
    let green_bold = green.add_modifier(Modifier::BOLD);
    let cyan_bold = if dimmed {
        Style::default()
            .fg(Color::DarkGray)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    };
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
    let path_popup_active = state.step == SetupStep::SelectPath && state.path_browse_mode;
    let blue = if path_popup_active {
        Style::default()
            .fg(Color::DarkGray)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
            .fg(Color::Rgb(37, 99, 235))
            .add_modifier(Modifier::BOLD)
    };
    let dim = Style::default().fg(Color::DarkGray);
    let yellow = if path_popup_active {
        Style::default().fg(Color::DarkGray)
    } else {
        Style::default().fg(Color::Yellow)
    };

    let top_center = match state.step {
        SetupStep::Welcome => vec![
            Span::styled("Press ", dim),
            Span::styled("[Enter]", blue),
            Span::styled(" to get started", dim),
        ],
        SetupStep::SelectProvider => vec![
            Span::styled("[↑] [↓]", blue),
            Span::styled(" Select provider", dim),
        ],
        SetupStep::Authenticate => {
            use super::state::AuthStatus;
            match &state.auth_status {
                AuthStatus::Pending | AuthStatus::Failed(_) => vec![
                    Span::styled("[Enter]", blue),
                    Span::styled(" Authenticate", dim),
                ],
                AuthStatus::Success => vec![
                    Span::styled("[Enter]", blue),
                    Span::styled(" Continue", dim),
                ],
                AuthStatus::Checking => vec![Span::styled("Authenticating...", yellow)],
            }
        }
        SetupStep::SelectPath => {
            if state.path_browse_mode {
                vec![Span::styled("Folder popup active", dim)]
            } else {
                vec![
                    Span::styled("[b]", blue),
                    Span::styled(" Open Folder Navigator", dim),
                ]
            }
        }
        SetupStep::SelectOrgs => {
            if state.org_loading {
                vec![Span::styled("Discovering organizations...", yellow)]
            } else {
                vec![
                    Span::styled("[Space]", blue),
                    Span::styled(" Toggle  ", dim),
                    Span::styled("[a]", blue),
                    Span::styled(" All  ", dim),
                    Span::styled("[n]", blue),
                    Span::styled(" None", dim),
                ]
            }
        }
        SetupStep::Confirm => vec![
            Span::styled("[Enter]", blue),
            Span::styled(" Save workspace", dim),
        ],
        SetupStep::Complete => vec![
            Span::styled("[Enter]", blue),
            Span::styled(" Dashboard  ", dim),
            Span::styled("[s]", blue),
            Span::styled(" Sync Now", dim),
        ],
    };

    let bottom_left = if path_popup_active {
        vec![
            Span::styled("[Esc]", blue),
            Span::styled(" Close Popup", dim),
        ]
    } else {
        vec![
            Span::styled("[q]", blue),
            Span::styled(" Quit  ", dim),
            Span::styled("[Esc]", blue),
            Span::styled(" Back", dim),
        ]
    };

    let bottom_right = match state.step {
        SetupStep::SelectProvider | SetupStep::SelectOrgs => vec![
            Span::styled("[↑] [↓]", blue),
            Span::styled(" Move  ", dim),
            Span::styled("[←] [→]", blue),
            Span::styled(" Step  ", dim),
            Span::styled("[Enter]", blue),
            Span::styled(" Next Step", dim),
        ],
        SetupStep::SelectPath => {
            if state.path_browse_mode {
                vec![Span::styled("Use popup arrows and Enter", dim)]
            } else {
                vec![
                    Span::styled("[←]", blue),
                    Span::styled(" Back Step  ", dim),
                    Span::styled("[Enter]", blue),
                    Span::styled(" Next Step  ", dim),
                    Span::styled("[b]", blue),
                    Span::styled(" Browse folders", dim),
                ]
            }
        }
        _ => vec![
            Span::styled("[←] [→]", blue),
            Span::styled(" Step  ", dim),
            Span::styled("[Enter]", blue),
            Span::styled(" Next Step", dim),
        ],
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
        Layout::horizontal([Constraint::Length(step_width), Constraint::Min(0)]).split(rows[0]);

    if let Some(text) = step_text {
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(text, dim))),
            top_cols[0],
        );
    }
    frame.render_widget(
        Paragraph::new(Line::from(top_center)).alignment(Alignment::Center),
        top_cols[1],
    );

    let bottom_cols =
        Layout::horizontal([Constraint::Length(24), Constraint::Min(0)]).split(rows[1]);
    frame.render_widget(Paragraph::new(Line::from(bottom_left)), bottom_cols[0]);
    frame.render_widget(
        Paragraph::new(Line::from(bottom_right)).right_aligned(),
        bottom_cols[1],
    );
}

#[cfg(test)]
#[path = "ui_tests.rs"]
mod tests;
