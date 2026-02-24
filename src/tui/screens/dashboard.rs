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
    render_actions(app, frame, chunks[3]);
    status_bar::render(
        frame,
        chunks[4],
        "q: Quit  s: Sync  t: Status  o: Orgs  w: Switch workspace  Enter: Menu",
    );
}

fn render_banner(frame: &mut Frame, area: Rect) {
    let style = Style::default()
        .fg(Color::Blue)
        .add_modifier(Modifier::BOLD);
    let banner_lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  ██████╗ ██╗████████╗   ███████╗ █████╗ ███╗   ███╗███████╗",
            style,
        )),
        Line::from(Span::styled(
            " ██╔════╝ ██║╚══██╔══╝   ██╔════╝██╔══██╗████╗ ████║██╔════╝",
            style,
        )),
        Line::from(Span::styled(
            " ██║  ███╗██║   ██║█████╗███████╗███████║██╔████╔██║█████╗  ",
            style,
        )),
        Line::from(Span::styled(
            " ██║   ██║██║   ██║╚════╝╚════██║██╔══██║██║╚██╔╝██║██╔══╝  ",
            style,
        )),
        Line::from(Span::styled(
            " ╚██████╔╝██║   ██║      ███████║██║  ██║██║ ╚═╝ ██║███████╗",
            style,
        )),
        Line::from(Span::styled(
            "  ╚═════╝ ╚═╝   ╚═╝      ╚══════╝╚═╝  ╚═╝╚═╝     ╚═╝╚══════╝",
            style,
        )),
    ];
    let banner = Paragraph::new(banner_lines).centered();
    frame.render_widget(banner, area);
}

fn render_info(app: &App, frame: &mut Frame, area: Rect) {
    let version = env!("CARGO_PKG_VERSION");

    let ws_info = match &app.active_workspace {
        Some(ws) => {
            let last = ws.last_synced.as_deref().unwrap_or("never");
            vec![
                Span::raw("  Workspace: "),
                Span::styled(&ws.name, Style::default().fg(Color::Cyan)),
                Span::styled(
                    format!("  Version {}", version),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::raw("  Path: "),
                Span::styled(&ws.base_path, Style::default().fg(Color::Cyan)),
                Span::raw("  Last synced: "),
                Span::styled(last, Style::default().fg(Color::DarkGray)),
            ]
        }
        None => vec![
            Span::styled(
                "  No workspace selected",
                Style::default().fg(Color::Yellow),
            ),
            Span::styled(
                format!("  Version {}", version),
                Style::default().fg(Color::DarkGray),
            ),
        ],
    };

    let info = Paragraph::new(vec![Line::from(ws_info)]).centered();
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

fn render_actions(app: &App, frame: &mut Frame, area: Rect) {
    let key = |k: &str| -> Span {
        Span::styled(
            format!("[{}]", k),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
    };

    let has_multiple_ws = app.workspaces.len() > 1;

    let mut lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::raw("  "),
            key("s"),
            Span::raw(" Sync   "),
            key("t"),
            Span::raw(" Status   "),
            key("o"),
            Span::raw(" Orgs"),
        ]),
    ];

    if has_multiple_ws {
        lines.push(Line::from(vec![
            Span::raw("  "),
            key("w"),
            Span::raw(" Switch workspace   "),
            key("Enter"),
            Span::raw(" Menu   "),
            key("q"),
            Span::raw(" Quit"),
        ]));
    } else {
        lines.push(Line::from(vec![
            Span::raw("  "),
            key("Enter"),
            Span::raw(" Menu   "),
            key("q"),
            Span::raw(" Quit"),
        ]));
    }

    let actions = Paragraph::new(lines).block(
        Block::default()
            .title(" Quick Actions ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    frame.render_widget(actions, area);
}
