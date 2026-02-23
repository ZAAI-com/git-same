//! Setup wizard render dispatcher.

use super::screens;
use super::state::{SetupState, SetupStep};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

/// Render the setup wizard.
pub fn render(state: &SetupState, frame: &mut Frame) {
    let area = frame.area();

    let chunks = Layout::vertical([
        Constraint::Length(3), // Header
        Constraint::Min(10),   // Content
    ])
    .split(area);

    render_header(state, frame, chunks[0]);

    match state.step {
        SetupStep::SelectProvider => screens::provider::render(state, frame, chunks[1]),
        SetupStep::Authenticate => screens::auth::render(state, frame, chunks[1]),
        SetupStep::SelectPath => screens::path::render(state, frame, chunks[1]),
        SetupStep::SelectOrgs => screens::orgs::render(state, frame, chunks[1]),
        SetupStep::Confirm => screens::confirm::render(state, frame, chunks[1]),
    }
}

/// Render the step progress header.
fn render_header(state: &SetupState, frame: &mut Frame, area: Rect) {
    let steps = [
        ("1", "Provider"),
        ("2", "Auth"),
        ("3", "Path"),
        ("4", "Orgs"),
        ("5", "Save"),
    ];

    let current_idx = match state.step {
        SetupStep::SelectProvider => 0,
        SetupStep::Authenticate => 1,
        SetupStep::SelectPath => 2,
        SetupStep::SelectOrgs => 3,
        SetupStep::Confirm => 4,
    };

    let mut spans = vec![Span::styled(
        " gisa setup  ",
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )];

    for (i, (num, label)) in steps.iter().enumerate() {
        let sep = if i > 0 { " › " } else { "" };
        let style = if i == current_idx {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else if i < current_idx {
            Style::default().fg(Color::Green)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        spans.push(Span::styled(sep, Style::default().fg(Color::DarkGray)));
        spans.push(Span::styled(format!("{} {}", num, label), style));
    }

    let header = Paragraph::new(Line::from(spans));
    frame.render_widget(header, area);
}
