//! Reusable repo table widget.

use ratatui::{
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Row, Table},
    Frame,
};

use crate::types::OwnedRepo;

/// Render a table of OwnedRepo entries.
pub fn render_owned_repos(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    repos: &[&OwnedRepo],
    selected: usize,
) {
    let header = Row::new(vec!["Name", "Default Branch", "Visibility"])
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .bottom_margin(1);

    let rows: Vec<Row> = repos
        .iter()
        .enumerate()
        .map(|(i, repo)| {
            let style = if i == selected {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            let visibility = if repo.repo.private {
                "private"
            } else {
                "public"
            };

            Row::new(vec![
                repo.repo.name.clone(),
                repo.repo.default_branch.clone(),
                visibility.to_string(),
            ])
            .style(style)
        })
        .collect();

    let widths = [
        Constraint::Percentage(50),
        Constraint::Percentage(25),
        Constraint::Percentage(25),
    ];

    let table = Table::new(rows, widths).header(header).block(
        Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    frame.render_widget(table, area);
}

#[cfg(test)]
#[path = "repo_table_tests.rs"]
mod tests;
