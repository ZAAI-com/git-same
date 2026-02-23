//! Org browser screen — two-pane: orgs list (left) + repos table (right).

use ratatui::{
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::tui::app::App;
use crate::tui::widgets::{repo_table, status_bar};

pub fn render(app: &App, frame: &mut Frame) {
    let chunks = Layout::vertical([
        Constraint::Min(1),    // Main content
        Constraint::Length(1), // Status bar
    ])
    .split(frame.area());

    let panes = Layout::horizontal([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(chunks[0]);

    render_org_list(app, frame, panes[0]);
    render_repo_list(app, frame, panes[1]);

    let hint = if app.filter_active {
        format!("Filter: {}|  Esc: Cancel", app.filter_text)
    } else {
        "j/k: Repos  J/K: Orgs  /: Filter  Esc: Back".to_string()
    };
    status_bar::render(frame, chunks[1], &hint);
}

fn render_org_list(app: &App, frame: &mut Frame, area: Rect) {
    if app.orgs.is_empty() {
        let empty = Paragraph::new("  No organizations discovered.\n  Run Clone or Fetch first.")
            .style(Style::default().fg(Color::DarkGray))
            .block(
                Block::default()
                    .title(" Organizations ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray)),
            );
        frame.render_widget(empty, area);
        return;
    }

    let items: Vec<ListItem> = app
        .orgs
        .iter()
        .enumerate()
        .map(|(i, org)| {
            let count = app.repos_by_org.get(org).map(|r| r.len()).unwrap_or(0);
            let marker = if i == app.org_index { ">" } else { " " };
            let style = if i == app.org_index {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(Line::from(vec![
                Span::styled(format!(" {} ", marker), style),
                Span::styled(org.clone(), style),
                Span::styled(
                    format!(" ({})", count),
                    Style::default().fg(Color::DarkGray),
                ),
            ]))
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .title(" Organizations ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    frame.render_widget(list, area);
}

fn render_repo_list(app: &App, frame: &mut Frame, area: Rect) {
    let selected_org = app.orgs.get(app.org_index);
    let title = selected_org
        .map(|o| format!(" Repositories ({}) ", o))
        .unwrap_or_else(|| " Repositories ".to_string());

    let repos = selected_org.and_then(|o| app.repos_by_org.get(o));

    match repos {
        Some(repos) if !repos.is_empty() => {
            let filtered: Vec<_> = if app.filter_text.is_empty() {
                repos.iter().collect()
            } else {
                let ft = app.filter_text.to_lowercase();
                repos
                    .iter()
                    .filter(|r| r.repo.name.to_lowercase().contains(&ft))
                    .collect()
            };

            repo_table::render_owned_repos(frame, area, &title, &filtered, app.repo_index);
        }
        _ => {
            let empty = Paragraph::new("  No repositories")
                .style(Style::default().fg(Color::DarkGray))
                .block(
                    Block::default()
                        .title(title)
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::DarkGray)),
                );
            frame.render_widget(empty, area);
        }
    }
}

use ratatui::layout::Rect;
