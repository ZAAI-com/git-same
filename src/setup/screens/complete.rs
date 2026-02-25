//! Step 6: Completion / success screen.

use crate::setup::state::SetupState;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

pub fn render(state: &SetupState, frame: &mut Frame, area: Rect) {
    let chunks = Layout::vertical([
        Constraint::Length(3), // Title
        Constraint::Min(10),   // Content
        Constraint::Length(2), // Help
    ])
    .split(area);

    // Title
    let title_text = if state.is_first_setup {
        "Workspace Created!"
    } else {
        "Workspace Added!"
    };
    let title = Paragraph::new(Line::from(vec![
        Span::styled(
            "  \u{2713} ",
            Style::default()
                .fg(Color::Rgb(21, 128, 61))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            title_text,
            Style::default()
                .fg(Color::Rgb(21, 128, 61))
                .add_modifier(Modifier::BOLD),
        ),
    ]));
    frame.render_widget(title, chunks[0]);

    // Summary
    let selected_orgs = state.selected_orgs();
    let total_repos: usize = state
        .orgs
        .iter()
        .filter(|o| o.selected)
        .map(|o| o.repo_count)
        .sum();
    let org_count = selected_orgs.len();

    let value_style = Style::default()
        .fg(Color::White)
        .add_modifier(Modifier::BOLD);
    let dim = Style::default().fg(Color::DarkGray);
    let yellow = Style::default().fg(Color::Yellow);

    let lines = vec![
        Line::raw(""),
        Line::from(Span::styled(
            format!("  {}", state.workspace_name),
            value_style,
        )),
        Line::from(Span::styled(format!("  {}", state.base_path), dim)),
        Line::from(Span::styled(
            format!(
                "  {} organization{}  \u{00b7}  {} repos",
                org_count,
                if org_count == 1 { "" } else { "s" },
                total_repos
            ),
            dim,
        )),
        Line::raw(""),
        Line::raw(""),
        Line::from(Span::styled("  Press Enter to continue", yellow)),
    ];

    let content = Paragraph::new(lines);
    frame.render_widget(content, chunks[1]);

    // Help
    let help = Paragraph::new("Enter Dashboard  Esc Back").style(dim);
    frame.render_widget(help, chunks[2]);
}
