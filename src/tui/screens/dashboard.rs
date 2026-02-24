//! Dashboard screen — home view with summary stats and quick-action hotkeys.

use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::tui::app::App;

pub fn render(app: &App, frame: &mut Frame) {
    let chunks = Layout::vertical([
        Constraint::Length(8), // Banner
        Constraint::Length(1), // Tagline + version
        Constraint::Length(1), // Config / requirements
        Constraint::Length(1), // Workspace info
        Constraint::Length(5), // Stats
        Constraint::Min(1),    // Spacer
        Constraint::Length(2), // Bottom actions (2 lines)
    ])
    .split(frame.area());

    render_banner(frame, chunks[0]);
    render_tagline(frame, chunks[1]);
    render_config_reqs(app, frame, chunks[2]);
    render_workspace_info(app, frame, chunks[3]);
    render_stats(app, frame, chunks[4]);
    render_bottom_actions(app, frame, chunks[6]);
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

fn render_tagline(frame: &mut Frame, area: Rect) {
    let version = env!("CARGO_PKG_VERSION");
    let description = env!("CARGO_PKG_DESCRIPTION");

    let line = Line::from(vec![
        Span::styled(description, Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("  v{}", version),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
    ]);
    let p = Paragraph::new(vec![line]).centered();
    frame.render_widget(p, area);
}

fn render_config_reqs(app: &App, frame: &mut Frame, area: Rect) {
    let dim = Style::default().fg(Color::DarkGray);
    let pass = Style::default().fg(Color::Green);
    let fail = Style::default().fg(Color::Red);
    let loading = Style::default().fg(Color::Yellow);

    let mut spans: Vec<Span> = Vec::new();

    if app.checks_loading {
        spans.push(Span::styled("Checking requirements...", loading));
    } else if app.check_results.is_empty() {
        spans.push(Span::styled("Requirements: checking...", dim));
    } else {
        for (i, check) in app.check_results.iter().enumerate() {
            if i > 0 {
                spans.push(Span::styled("  ", dim));
            }
            let icon = if check.passed { "✓" } else { "✗" };
            let style = if check.passed { pass } else { fail };
            spans.push(Span::styled(&check.name, dim));
            spans.push(Span::raw(" "));
            spans.push(Span::styled(icon, style));
        }

        spans.push(Span::styled("  │  ", dim));
        spans.push(Span::styled(
            format!("Concurrency: {}", app.config.concurrency),
            dim,
        ));
    }

    let p = Paragraph::new(vec![Line::from(spans)]).centered();
    frame.render_widget(p, area);
}

fn render_workspace_info(app: &App, frame: &mut Frame, area: Rect) {
    let dim = Style::default().fg(Color::DarkGray);
    let cyan = Style::default().fg(Color::Cyan);
    let sep = Span::styled("  │  ", dim);

    let spans = match &app.active_workspace {
        Some(ws) => {
            let org_count = if ws.orgs.is_empty() {
                "all orgs".to_string()
            } else {
                format!("{} org(s)", ws.orgs.len())
            };
            let last = ws.last_synced.as_deref().unwrap_or("never");
            let provider = ws.provider.kind.display_name();

            vec![
                Span::styled("Workspace: ", dim),
                Span::styled(&ws.name, cyan),
                sep.clone(),
                Span::styled("Path: ", dim),
                Span::styled(&ws.base_path, cyan),
                sep.clone(),
                Span::styled(format!("Provider: {}", provider), dim),
                sep.clone(),
                Span::styled(format!("Orgs: {}", org_count), dim),
                sep,
                Span::styled(format!("Last synced: {}", last), dim),
            ]
        }
        None => vec![Span::styled(
            "No workspace selected",
            Style::default().fg(Color::Yellow),
        )],
    };

    let p = Paragraph::new(vec![Line::from(spans)]).centered();
    frame.render_widget(p, area);
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

    let selected = app.stat_index;
    render_stat_box(
        frame,
        cols[0],
        &total_orgs.to_string(),
        "Orgs",
        Color::Cyan,
        selected == 0,
    );
    render_stat_box(
        frame,
        cols[1],
        &total_repos.to_string(),
        "Repos",
        Color::Cyan,
        selected == 1,
    );
    render_stat_box(
        frame,
        cols[2],
        &dirty.to_string(),
        "Dirty",
        Color::Yellow,
        selected == 2,
    );
    render_stat_box(
        frame,
        cols[3],
        &behind.to_string(),
        "Behind",
        Color::Red,
        selected == 3,
    );
    render_stat_box(
        frame,
        cols[4],
        &clean.to_string(),
        "Clean",
        Color::Green,
        selected == 4,
    );
    render_stat_box(
        frame,
        cols[5],
        &ahead.to_string(),
        "Ahead",
        Color::Blue,
        selected == 5,
    );
}

fn render_stat_box(
    frame: &mut Frame,
    area: Rect,
    value: &str,
    label: &str,
    color: Color,
    selected: bool,
) {
    let border_color = if selected { color } else { Color::DarkGray };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));
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

fn render_bottom_actions(app: &App, frame: &mut Frame, area: Rect) {
    let rows = Layout::vertical([
        Constraint::Length(1), // Actions
        Constraint::Length(1), // Navigation
    ])
    .split(area);

    let dim = Style::default().fg(Color::DarkGray);
    let key_style = Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD);

    // Line 1: Actions
    let actions = Line::from(vec![
        Span::raw(" "),
        Span::styled("[s]", key_style),
        Span::styled(" Sync", dim),
        Span::raw("   "),
        Span::styled("[t]", key_style),
        Span::styled(" Status", dim),
        Span::raw("   "),
        Span::styled("[o]", key_style),
        Span::styled(" Orgs", dim),
        Span::raw("   "),
        Span::styled("[e]", key_style),
        Span::styled(" Settings", dim),
        Span::raw("   "),
        Span::styled("[c]", key_style),
        Span::styled(" Config", dim),
        Span::raw("   "),
        Span::styled("[m]", key_style),
        Span::styled(" Menu", dim),
    ]);

    // Line 2: Navigation
    let has_multiple_ws = app.workspaces.len() > 1;
    let mut nav_spans = vec![
        Span::raw(" "),
        Span::styled("[q]", key_style),
        Span::styled(" Quit", dim),
        Span::raw("   "),
        Span::styled("[Esc]", key_style),
        Span::styled(" Back", dim),
        Span::raw("   "),
        Span::styled("[←]", key_style),
        Span::styled(" Left", dim),
        Span::raw("   "),
        Span::styled("[→]", key_style),
        Span::styled(" Right", dim),
        Span::raw("   "),
        Span::styled("[↵]", key_style),
        Span::styled(" Select", dim),
    ];
    if has_multiple_ws {
        nav_spans.push(Span::raw("   "));
        nav_spans.push(Span::styled("[w]", key_style));
        nav_spans.push(Span::styled(" Workspace", dim));
    }
    let navigation = Line::from(nav_spans);

    let actions_p = Paragraph::new(vec![actions]).centered();
    let nav_p = Paragraph::new(vec![navigation]).centered();

    frame.render_widget(actions_p, rows[0]);
    frame.render_widget(nav_p, rows[1]);
}
