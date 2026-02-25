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
        SetupStep::SelectPath => screens::path::render(state, frame, content_area),
        SetupStep::SelectOrgs => screens::orgs::render(state, frame, content_area),
        SetupStep::Confirm => screens::confirm::render(state, frame, content_area),
        SetupStep::Complete => screens::complete::render(state, frame, content_area),
    }

    // Status bar
    render_status_bar(state, frame, chunks[idx]);
}

/// Render the step progress indicator with nodes and connectors.
fn render_step_progress(state: &SetupState, frame: &mut Frame, area: Rect) {
    let steps = ["Provider", "Auth", "Path", "Orgs", "Save"];
    let current = state.step_number(); // 0 for Welcome, 1-5 for steps, 5 for Complete

    let green = Style::default().fg(Color::Rgb(21, 128, 61));
    let green_bold = green.add_modifier(Modifier::BOLD);
    let cyan_bold = Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD);
    let dim = Style::default().fg(Color::DarkGray);

    // Line 1: nodes and connectors
    let mut node_spans: Vec<Span> = Vec::new();
    node_spans.push(Span::raw("   "));

    for (i, _label) in steps.iter().enumerate() {
        let step_num = i + 1;

        if i > 0 {
            // Connector between nodes
            if step_num <= current {
                node_spans.push(Span::styled(" \u{2501}\u{2501}\u{2501} ", green));
            } else {
                node_spans.push(Span::styled(" \u{2500} \u{2500} ", dim));
            }
        }

        // Node
        if step_num < current || state.step == SetupStep::Complete {
            // Completed: green checkmark
            node_spans.push(Span::styled("(\u{2713})", green_bold));
        } else if step_num == current {
            // Active: cyan number
            node_spans.push(Span::styled(format!("({})", step_num), cyan_bold));
        } else {
            // Upcoming: dim number
            node_spans.push(Span::styled(format!("({})", step_num), dim));
        }
    }

    // Line 2: labels under nodes
    let mut label_spans: Vec<Span> = Vec::new();
    label_spans.push(Span::raw("  "));

    for (i, label) in steps.iter().enumerate() {
        let step_num = i + 1;

        if i > 0 {
            label_spans.push(Span::raw("     "));
        }

        let style = if step_num < current || state.step == SetupStep::Complete {
            green
        } else if step_num == current {
            cyan_bold
        } else {
            dim
        };

        label_spans.push(Span::styled(format!("{:<8}", label), style));
    }

    let lines = vec![Line::from(node_spans), Line::from(label_spans)];

    let widget = Paragraph::new(lines).alignment(Alignment::Center);
    frame.render_widget(widget, area);
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
                Span::styled("[qq]", blue),
                Span::styled(" Quit", dim),
            ],
        ),
        SetupStep::SelectProvider => (
            vec![Span::styled(" [Enter]", blue), Span::styled(" Select", dim)],
            vec![
                Span::styled(" [j/k]", blue),
                Span::styled(" Navigate  ", dim),
                Span::styled("[Esc]", blue),
                Span::styled(" Cancel  ", dim),
                Span::styled("[qq]", blue),
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
                    Span::styled("[qq]", blue),
                    Span::styled(" Quit", dim),
                ],
            )
        }
        SetupStep::SelectPath => {
            if state.path_suggestions_mode {
                (
                    vec![
                        Span::styled(" [Enter]", blue),
                        Span::styled(" Confirm  ", dim),
                        Span::styled("[Tab]", blue),
                        Span::styled(" Edit", dim),
                    ],
                    vec![
                        Span::styled(" [j/k]", blue),
                        Span::styled(" Select  ", dim),
                        Span::styled("[Esc]", blue),
                        Span::styled(" Back  ", dim),
                        Span::styled("[qq]", blue),
                        Span::styled(" Quit", dim),
                    ],
                )
            } else {
                (
                    vec![
                        Span::styled(" [Enter]", blue),
                        Span::styled(" Confirm  ", dim),
                        Span::styled("[Tab]", blue),
                        Span::styled(" Complete", dim),
                    ],
                    vec![
                        Span::styled(" [Esc]", blue),
                        Span::styled(" Back  ", dim),
                        Span::styled("[qq]", blue),
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
                        Span::styled("[qq]", blue),
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
                        Span::styled(" [j/k]", blue),
                        Span::styled(" Navigate  ", dim),
                        Span::styled("[Esc]", blue),
                        Span::styled(" Back  ", dim),
                        Span::styled("[qq]", blue),
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
                Span::styled("[qq]", blue),
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
                Span::styled("[qq]", blue),
                Span::styled(" Quit", dim),
            ],
        ),
    };

    // Add step counter to actions line (right-aligned)
    let step_num = state.step_number();
    let mut actions_with_step = actions;
    if step_num > 0 {
        let step_text = format!("Step {} of {}", step_num, SetupState::TOTAL_STEPS);
        let used_width: usize = actions_with_step.iter().map(|s| s.width()).sum();
        let available = area.width as usize;
        if available > used_width + step_text.len() + 2 {
            let pad = available - used_width - step_text.len() - 1;
            actions_with_step.push(Span::raw(" ".repeat(pad)));
            actions_with_step.push(Span::styled(step_text, dim));
        }
    }

    let lines = vec![Line::from(actions_with_step), Line::from(nav)];

    let widget = Paragraph::new(lines);
    frame.render_widget(widget, area);
}
