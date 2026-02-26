//! Step 3: Organization selection screen with summary and proportional bars.

use crate::setup::state::SetupState;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

/// Braille spinner frames (same as auth).
const SPINNER: [char; 10] = [
    '\u{280b}', '\u{2819}', '\u{2839}', '\u{2838}', '\u{283c}', '\u{2834}', '\u{2826}', '\u{2827}',
    '\u{2807}', '\u{280f}',
];

pub fn render(state: &SetupState, frame: &mut Frame, area: Rect) {
    let mut lines: Vec<Line> = Vec::new();

    // Title
    let selected_count = state.orgs.iter().filter(|o| o.selected).count();
    let total_repos: usize = state.orgs.iter().map(|o| o.repo_count).sum();
    let selected_repos: usize = state
        .orgs
        .iter()
        .filter(|o| o.selected)
        .map(|o| o.repo_count)
        .sum();

    lines.push(Line::from(Span::styled(
        "  Select organizations to sync",
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )));

    if !state.orgs.is_empty() {
        lines.push(Line::from(vec![
            Span::styled(
                format!("  {} of {} selected", selected_count, state.orgs.len()),
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled("  \u{00b7}  ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{} repos", selected_repos),
                Style::default().fg(Color::Rgb(21, 128, 61)),
            ),
            Span::styled(
                format!(" of {} total", total_repos),
                Style::default().fg(Color::DarkGray),
            ),
        ]));
    }
    lines.push(Line::raw(""));

    // Content
    if state.org_loading {
        let spinner_char = SPINNER[(state.tick_count as usize) % SPINNER.len()];
        lines.push(Line::from(Span::styled(
            format!("  {} Discovering organizations...", spinner_char),
            Style::default().fg(Color::Yellow),
        )));
    } else if let Some(ref err) = state.org_error {
        lines.push(Line::from(Span::styled(
            "  \u{2717} Failed to discover organizations",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::raw(""));
        lines.push(Line::from(Span::styled(
            format!("  {}", err),
            Style::default().fg(Color::White),
        )));
        lines.push(Line::raw(""));
        lines.push(Line::from(Span::styled(
            "  Press Enter to retry",
            Style::default().fg(Color::Yellow),
        )));
    } else if state.orgs.is_empty() {
        lines.push(Line::from(Span::styled(
            "  No organizations found. Press Enter to continue.",
            Style::default().fg(Color::DarkGray),
        )));
        lines.push(Line::from(Span::styled(
            "  Your personal repos will still be synced.",
            Style::default().fg(Color::DarkGray),
        )));
    } else {
        let max_repos = state.orgs.iter().map(|o| o.repo_count).max().unwrap_or(1);
        let bar_width = 16;

        for (i, org) in state.orgs.iter().enumerate() {
            let is_selected = i == state.org_index;
            let marker = if is_selected { " \u{25b8}" } else { "  " };
            let checkbox = if org.selected { "[x]" } else { "[ ]" };

            let green = Color::Rgb(21, 128, 61);

            let (marker_style, name_style, count_style) = if is_selected {
                (
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                    Style::default().fg(Color::DarkGray),
                )
            } else if org.selected {
                (
                    Style::default().fg(green),
                    Style::default().fg(Color::White),
                    Style::default().fg(Color::DarkGray),
                )
            } else {
                (
                    Style::default().fg(Color::White),
                    Style::default().fg(Color::White),
                    Style::default().fg(Color::DarkGray),
                )
            };

            // Proportional bar
            let filled = if max_repos > 0 {
                (org.repo_count * bar_width) / max_repos
            } else {
                0
            }
            .max(if org.repo_count > 0 { 1 } else { 0 });
            let empty = bar_width - filled;

            let bar_color = if org.selected { green } else { Color::DarkGray };

            let mut spans = vec![
                Span::styled(format!("{} {} ", marker, checkbox), marker_style),
                Span::styled(format!("{:<20}", org.name), name_style),
                Span::styled(format!("{:>4} repos  ", org.repo_count), count_style),
                Span::styled("\u{2588}".repeat(filled), Style::default().fg(bar_color)),
                Span::styled(
                    "\u{2591}".repeat(empty),
                    Style::default().fg(Color::DarkGray),
                ),
            ];

            // Percentage
            if total_repos > 0 {
                let pct = (org.repo_count * 100) / total_repos;
                spans.push(Span::styled(
                    format!(" {:>3}%", pct),
                    Style::default().fg(Color::DarkGray),
                ));
            }

            lines.push(Line::from(spans));
        }
    }

    let widget = Paragraph::new(lines);
    frame.render_widget(widget, area);
}
