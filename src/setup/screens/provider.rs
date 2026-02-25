//! Step 1: Provider selection screen with descriptions.

use crate::setup::state::SetupState;
use crate::types::ProviderKind;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

/// Get a short description for each provider.
fn provider_description(kind: ProviderKind) -> &'static str {
    match kind {
        ProviderKind::GitHub => "github.com \u{2014} Public and private repositories",
        ProviderKind::GitHubEnterprise => "Self-hosted GitHub instance",
        ProviderKind::GitLab => "gitlab.com or self-hosted",
        ProviderKind::Bitbucket => "bitbucket.org",
    }
}

pub fn render(state: &SetupState, frame: &mut Frame, area: Rect) {
    let mut lines: Vec<Line> = Vec::new();

    // Title
    lines.push(Line::from(Span::styled(
        "  Select your Git provider",
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::raw(""));

    // Provider list with descriptions
    for (i, choice) in state.provider_choices.iter().enumerate() {
        let is_selected = i == state.provider_index;
        let marker = if is_selected { "  \u{25b8}  " } else { "     " };

        let (label_style, desc_style) = if !choice.available {
            (
                Style::default().fg(Color::DarkGray),
                Style::default().fg(Color::DarkGray),
            )
        } else if is_selected {
            (
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
                Style::default().fg(Color::White),
            )
        } else {
            (
                Style::default().fg(Color::White),
                Style::default().fg(Color::DarkGray),
            )
        };

        lines.push(Line::from(vec![
            Span::styled(marker, label_style),
            Span::styled(&choice.label, label_style),
        ]));

        // Description line
        lines.push(Line::from(Span::styled(
            format!("        {}", provider_description(choice.kind)),
            desc_style,
        )));

        lines.push(Line::raw(""));
    }

    let widget = Paragraph::new(lines);
    frame.render_widget(widget, area);
}
