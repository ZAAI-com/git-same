//! Dashboard screen — home view with summary stats and quick-action hotkeys.

use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::tui::app::App;
use crate::tui::widgets::status_bar;

pub fn render(app: &App, frame: &mut Frame) {
    let chunks = Layout::vertical([
        Constraint::Length(8), // Banner
        Constraint::Length(3), // Info
        Constraint::Length(5), // Stats
        Constraint::Min(4),    // Quick actions
        Constraint::Length(1), // Status bar
    ])
    .split(frame.area());

    render_banner(frame, chunks[0]);
    render_info(app, frame, chunks[1]);
    render_stats(app, frame, chunks[2]);
    render_actions(frame, chunks[3]);
    status_bar::render(
        frame,
        chunks[4],
        "q: Quit  c: Clone  f: Fetch  p: Pull  s: Status  o: Orgs  Enter: Menu",
    );
}

fn render_banner(frame: &mut Frame, area: Rect) {
    let banner_lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  ██████╗ ██╗███████╗ █████╗ ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            " ██╔════╝ ██║██╔════╝██╔══██╗",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            " ██║  ███╗██║███████╗███████║",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            " ██║   ██║██║╚════██║██╔══██║",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            " ╚██████╔╝██║███████║██║  ██║",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            "  ╚═════╝ ╚═╝╚══════╝╚═╝  ╚═╝",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
    ];
    let banner = Paragraph::new(banner_lines).centered();
    frame.render_widget(banner, area);
}

fn render_info(app: &App, frame: &mut Frame, area: Rect) {
    let base = app
        .base_path
        .as_ref()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "(not set)".to_string());

    let version = env!("CARGO_PKG_VERSION");

    let info = Paragraph::new(vec![Line::from(vec![
        Span::styled(
            "  Mirror GitHub, locally. ",
            Style::default().fg(Color::DarkGray),
        ),
        Span::styled(
            format!("v{}  ", version),
            Style::default().fg(Color::DarkGray),
        ),
        Span::raw("  Base: "),
        Span::styled(base, Style::default().fg(Color::Cyan)),
    ])])
    .centered();
    frame.render_widget(info, area);
}

fn render_stats(app: &App, frame: &mut Frame, area: Rect) {
    let cols = Layout::horizontal([
        Constraint::Ratio(1, 6),
        Constraint::Ratio(1, 6),
        Constraint::Ratio(1, 6),
        Constraint::Ratio(1, 6),
        Constraint::Ratio(1, 6),
        Constraint::Ratio(1, 6),
    ])
    .split(area);

    let total_repos = app.all_repos.len();
    let total_orgs = app.orgs.len();
    let dirty = app.local_repos.iter().filter(|r| r.is_dirty).count();
    let behind = app.local_repos.iter().filter(|r| r.behind > 0).count();
    let ahead = app.local_repos.iter().filter(|r| r.ahead > 0).count();
    let clean = app
        .local_repos
        .iter()
        .filter(|r| !r.is_dirty && r.behind == 0 && r.ahead == 0)
        .count();

    render_stat_box(frame, cols[0], &total_orgs.to_string(), "Orgs", Color::Cyan);
    render_stat_box(
        frame,
        cols[1],
        &total_repos.to_string(),
        "Repos",
        Color::Cyan,
    );
    render_stat_box(frame, cols[2], &dirty.to_string(), "Dirty", Color::Yellow);
    render_stat_box(frame, cols[3], &behind.to_string(), "Behind", Color::Red);
    render_stat_box(frame, cols[4], &clean.to_string(), "Clean", Color::Green);
    render_stat_box(frame, cols[5], &ahead.to_string(), "Ahead", Color::Blue);
}

fn render_stat_box(frame: &mut Frame, area: Rect, value: &str, label: &str, color: Color) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));
    let content = Paragraph::new(vec![
        Line::from(Span::styled(
            value,
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(label, Style::default().fg(Color::DarkGray))),
    ])
    .centered()
    .block(block);
    frame.render_widget(content, area);
}

fn render_actions(frame: &mut Frame, area: Rect) {
    let actions = Paragraph::new(vec![
        Line::from(""),
        Line::from(vec![
            Span::raw("  "),
            Span::styled(
                "[c]",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" Clone   "),
            Span::styled(
                "[f]",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" Fetch   "),
            Span::styled(
                "[p]",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" Pull   "),
            Span::styled(
                "[s]",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" Status"),
        ]),
        Line::from(vec![
            Span::raw("  "),
            Span::styled(
                "[o]",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" Orgs    "),
            Span::styled(
                "[Enter]",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" Menu  "),
            Span::styled(
                "[q]",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" Quit"),
        ]),
    ])
    .block(
        Block::default()
            .title(" Quick Actions ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    frame.render_widget(actions, area);
}
