//! Repo status screen — filterable table of all local repos.

use ratatui::{
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Row, Table},
    Frame,
};

use crate::tui::app::App;
use crate::tui::widgets::status_bar;

pub fn render(app: &App, frame: &mut Frame) {
    let chunks = Layout::vertical([
        Constraint::Length(3), // Title + filter
        Constraint::Min(5),    // Table
        Constraint::Length(1), // Status bar
    ])
    .split(frame.area());

    render_header(app, frame, chunks[0]);
    render_table(app, frame, chunks[1]);

    let hint = if app.filter_active {
        format!("Filter: {}|  Esc: Cancel  Enter: Apply", app.filter_text)
    } else {
        "j/k: Navigate  /: Filter  D: Uncommitted  B: Behind  r: Refresh  Esc: Back".to_string()
    };
    status_bar::render(frame, chunks[2], &hint);
}

fn render_header(app: &App, frame: &mut Frame, area: ratatui::layout::Rect) {
    let filtered = filtered_repos(app);
    let total = app.local_repos.len();

    let mut spans = vec![
        Span::styled(
            " Repository Status ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!("  Showing: {}/{}", filtered.len(), total)),
    ];

    if app.filter_uncommitted {
        spans.push(Span::styled(
            "  [Uncommitted]",
            Style::default().fg(Color::Yellow),
        ));
    }
    if app.filter_behind {
        spans.push(Span::styled("  [Behind]", Style::default().fg(Color::Red)));
    }

    let header = Paragraph::new(Line::from(spans)).block(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    frame.render_widget(header, area);
}

fn render_table(app: &App, frame: &mut Frame, area: ratatui::layout::Rect) {
    if app.status_loading {
        let loading = Paragraph::new(Line::from(Span::styled(
            "  Scanning repositories...",
            Style::default().fg(Color::Yellow),
        )))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        );
        frame.render_widget(loading, area);
        return;
    }

    let repos = filtered_repos(app);

    let header = Row::new(vec!["Org/Repo", "Branch", "Uncommitted", "Ahead", "Behind"])
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .bottom_margin(1);

    let rows: Vec<Row> = repos
        .iter()
        .enumerate()
        .map(|(i, entry)| {
            let style = if i == app.repo_index {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            let branch = entry.branch.as_deref().unwrap_or("-");
            let uncommitted = if entry.is_uncommitted { "*" } else { "." };
            let ahead = if entry.ahead > 0 {
                format!("+{}", entry.ahead)
            } else {
                ".".to_string()
            };
            let behind = if entry.behind > 0 {
                format!("-{}", entry.behind)
            } else {
                ".".to_string()
            };

            Row::new(vec![
                entry.full_name.clone(),
                branch.to_string(),
                uncommitted.to_string(),
                ahead,
                behind,
            ])
            .style(style)
        })
        .collect();

    let widths = [
        Constraint::Percentage(40),
        Constraint::Percentage(20),
        Constraint::Percentage(10),
        Constraint::Percentage(15),
        Constraint::Percentage(15),
    ];

    let table = Table::new(rows, widths).header(header).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    frame.render_widget(table, area);
}

fn filtered_repos(app: &App) -> Vec<&crate::tui::app::RepoEntry> {
    app.local_repos
        .iter()
        .filter(|r| {
            if app.filter_uncommitted && !r.is_uncommitted {
                return false;
            }
            if app.filter_behind && r.behind == 0 {
                return false;
            }
            if !app.filter_text.is_empty()
                && !r
                    .full_name
                    .to_lowercase()
                    .contains(&app.filter_text.to_lowercase())
            {
                return false;
            }
            true
        })
        .collect()
}
