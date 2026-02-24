//! Dashboard screen — home view with summary stats and quick-action hotkeys.

use std::collections::HashSet;

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
        Constraint::Length(7), // Banner
        Constraint::Length(1), // Tagline + version
        Constraint::Length(1), // Requirements status
        Constraint::Length(1), // Workspace info line 1
        Constraint::Length(1), // Workspace info line 2
        Constraint::Length(5), // Stats
        Constraint::Min(1),    // Spacer
        Constraint::Length(2), // Bottom actions (2 lines)
    ])
    .split(frame.area());

    render_banner(frame, chunks[0]);
    render_tagline(frame, chunks[1]);
    render_config_reqs(app, frame, chunks[2]);
    render_workspace_info(app, frame, chunks[3], chunks[4]);
    render_stats(app, frame, chunks[5]);
    render_bottom_actions(app, frame, chunks[7]);
}

fn render_banner(frame: &mut Frame, area: Rect) {
    let lines = [
        "  ██████╗ ██╗████████╗   ███████╗ █████╗ ███╗   ███╗███████╗",
        " ██╔════╝ ██║╚══██╔══╝   ██╔════╝██╔══██╗████╗ ████║██╔════╝",
        " ██║  ███╗██║   ██║█████╗███████╗███████║██╔████╔██║█████╗  ",
        " ██║   ██║██║   ██║╚════╝╚════██║██╔══██║██║╚██╔╝██║██╔══╝  ",
        " ╚██████╔╝██║   ██║      ███████║██║  ██║██║ ╚═╝ ██║███████╗",
        "  ╚═════╝ ╚═╝   ╚═╝      ╚══════╝╚═╝  ╚═╝╚═╝     ╚═╝╚══════╝",
    ];
    let stops: [(u8, u8, u8); 4] = [
        (168, 85, 247), // Purple
        (59, 130, 246), // Blue
        (6, 182, 212),  // Cyan
        (34, 197, 94),  // Green
    ];
    let mut banner_lines: Vec<Line> = Vec::new();
    for text in &lines {
        banner_lines.push(gradient_line(text, &stops));
    }
    let banner = Paragraph::new(banner_lines).centered();
    frame.render_widget(banner, area);
}

fn gradient_line<'a>(text: &'a str, stops: &[(u8, u8, u8)]) -> Line<'a> {
    let chars: Vec<&str> = text.split_inclusive(|_: char| true).collect();
    let len = chars.len().max(1);
    let spans: Vec<Span<'a>> = chars
        .into_iter()
        .enumerate()
        .map(|(i, ch)| {
            let t = i as f64 / (len - 1).max(1) as f64;
            let (r, g, b) = interpolate_stops(stops, t);
            Span::styled(
                ch.to_string(),
                Style::default()
                    .fg(Color::Rgb(r, g, b))
                    .add_modifier(Modifier::BOLD),
            )
        })
        .collect();
    Line::from(spans)
}

fn interpolate_stops(stops: &[(u8, u8, u8)], t: f64) -> (u8, u8, u8) {
    let t = t.clamp(0.0, 1.0);
    let segments = stops.len() - 1;
    let scaled = t * segments as f64;
    let idx = (scaled as usize).min(segments - 1);
    let local_t = scaled - idx as f64;
    let (r1, g1, b1) = stops[idx];
    let (r2, g2, b2) = stops[idx + 1];
    let lerp = |a: u8, b: u8, t: f64| -> u8 { (a as f64 + (b as f64 - a as f64) * t) as u8 };
    (lerp(r1, r2, local_t), lerp(g1, g2, local_t), lerp(b1, b2, local_t))
}

fn render_tagline(frame: &mut Frame, area: Rect) {
    let version = env!("CARGO_PKG_VERSION");
    let description = env!("CARGO_PKG_DESCRIPTION");

    let line = Line::from(vec![
        Span::styled(description, Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("  Version {}", version),
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
    let key_style = Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD);
    let loading_style = Style::default().fg(Color::Yellow);

    let mut spans: Vec<Span> = Vec::new();

    if app.checks_loading || app.check_results.is_empty() {
        spans.push(Span::styled("Checking requirements...", loading_style));
    } else {
        let all_passed = app.check_results.iter().all(|c| c.passed);
        if all_passed {
            spans.push(Span::styled(
                "Requirements ✓",
                Style::default().fg(Color::Green),
            ));
            spans.push(Span::styled("   ", dim));
            spans.push(Span::styled("[e]", key_style));
            spans.push(Span::styled(" Settings", dim));
        } else {
            spans.push(Span::styled(
                "Requirements ✗",
                Style::default().fg(Color::Red),
            ));
            spans.push(Span::styled("   ", dim));
            spans.push(Span::styled("[i]", key_style));
            spans.push(Span::styled(" Init", dim));
        }
    }

    let p = Paragraph::new(vec![Line::from(spans)]).centered();
    frame.render_widget(p, area);
}

fn render_workspace_info(app: &App, frame: &mut Frame, line1: Rect, line2: Rect) {
    let dim = Style::default().fg(Color::DarkGray);
    let cyan = Style::default().fg(Color::Cyan);
    let sep = Span::styled("  │  ", dim);

    match &app.active_workspace {
        Some(ws) => {
            let last = ws.last_synced.as_deref().unwrap_or("never");
            let provider = ws.provider.kind.display_name();

            // Line 1: Workspace name + path
            let top = Line::from(vec![
                Span::styled("Workspace: ", dim),
                Span::styled(&ws.name, cyan),
                sep.clone(),
                Span::styled("Path: ", dim),
                Span::styled(&ws.base_path, cyan),
            ]);

            // Line 2: Provider + last synced
            let bottom = Line::from(vec![
                Span::styled(format!("Provider: {}", provider), dim),
                sep,
                Span::styled(format!("Last synced: {}", last), dim),
            ]);

            frame.render_widget(Paragraph::new(vec![top]).centered(), line1);
            frame.render_widget(Paragraph::new(vec![bottom]).centered(), line2);
        }
        None => {
            let p = Paragraph::new(vec![Line::from(Span::styled(
                "No workspace selected",
                Style::default().fg(Color::Yellow),
            ))])
            .centered();
            frame.render_widget(p, line1);
        }
    }
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

    let total_repos = app.local_repos.len();
    let total_orgs = app
        .local_repos
        .iter()
        .map(|r| r.owner.as_str())
        .collect::<HashSet<_>>()
        .len();
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

fn render_bottom_actions(_app: &App, frame: &mut Frame, area: Rect) {
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
        Span::styled("[m]", key_style),
        Span::styled(" Menu", dim),
    ]);

    // Line 2: Navigation — left-aligned (Quit, Back) and right-aligned (Left, Right, Select)
    let nav_cols =
        Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)]).split(rows[1]);

    let left_spans = vec![
        Span::raw(" "),
        Span::styled("[q]", key_style),
        Span::styled(" Quit", dim),
        Span::raw("   "),
        Span::styled("[Esc]", key_style),
        Span::styled(" Back", dim),
        Span::raw("   "),
        Span::styled("[w]", key_style),
        Span::styled(" Workspace", dim),
    ];

    let right_spans = vec![
        Span::styled("[←]", key_style),
        Span::styled(" Left", dim),
        Span::raw("   "),
        Span::styled("[→]", key_style),
        Span::styled(" Right", dim),
        Span::raw("   "),
        Span::styled("[↵]", key_style),
        Span::styled(" Select", dim),
        Span::raw(" "),
    ];

    let actions_p = Paragraph::new(vec![actions]).centered();
    let nav_left = Paragraph::new(vec![Line::from(left_spans)]);
    let nav_right = Paragraph::new(vec![Line::from(right_spans)]).right_aligned();

    frame.render_widget(actions_p, rows[0]);
    frame.render_widget(nav_left, nav_cols[0]);
    frame.render_widget(nav_right, nav_cols[1]);
}
