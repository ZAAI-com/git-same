//! Dashboard screen — home view with summary stats and quick-action hotkeys.

use std::collections::{HashMap, HashSet};

use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Row, Table},
    Frame,
};

use chrono::DateTime;

use crate::tui::app::{App, RepoEntry};

pub(crate) fn format_timestamp(raw: &str) -> String {
    use chrono::Utc;

    let parsed = DateTime::parse_from_rfc3339(raw);
    match parsed {
        Ok(dt) => {
            let absolute = dt.format("%Y-%m-%d %H:%M:%S").to_string();
            let duration = Utc::now().signed_duration_since(dt);
            let relative = if duration.num_days() > 30 {
                format!("about {}mo ago", duration.num_days() / 30)
            } else if duration.num_days() > 0 {
                format!("about {}d ago", duration.num_days())
            } else if duration.num_hours() > 0 {
                format!("about {}h ago", duration.num_hours())
            } else if duration.num_minutes() > 0 {
                format!("about {} min ago", duration.num_minutes())
            } else {
                "just now".to_string()
            };
            format!("{} at {}", relative, absolute)
        }
        Err(_) => raw.to_string(),
    }
}

pub fn render(app: &App, frame: &mut Frame) {
    let chunks = Layout::vertical([
        Constraint::Length(6), // Banner
        Constraint::Length(1), // Tagline + version
        Constraint::Length(1), // Requirements status
        Constraint::Length(1), // Workspace info line 1
        Constraint::Length(1), // Workspace info line 2
        Constraint::Length(4), // Stats
        Constraint::Min(1),    // Spacer
        Constraint::Length(2), // Bottom actions (2 lines)
    ])
    .split(frame.area());

    render_banner(frame, chunks[0]);
    render_tagline(frame, chunks[1]);
    render_config_reqs(app, frame, chunks[2]);
    render_workspace_info(app, frame, chunks[3], chunks[4]);
    render_stats(app, frame, chunks[5]);
    render_tab_content(app, frame, chunks[6]);
    render_bottom_actions(app, frame, chunks[7]);
}

fn render_banner(frame: &mut Frame, area: Rect) {
    let lines = [
        " ██████╗ ██╗████████╗   ███████╗ █████╗ ███╗   ███╗███████╗",
        "██╔════╝ ██║╚══██╔══╝   ██╔════╝██╔══██╗████╗ ████║██╔════╝",
        "██║  ███╗██║   ██║█████╗███████╗███████║██╔████╔██║█████╗  ",
        "██║   ██║██║   ██║╚════╝╚════██║██╔══██║██║╚██╔╝██║██╔══╝  ",
    ];
    // Line 5: E bottom bar has version embedded with inverted colors
    let line5_prefix = "╚██████╔╝██║   ██║      ███████║██║  ██║██║ ╚═╝ ██║█";
    let line5_suffix = "╗";
    let last_line = " ╚═════╝ ╚═╝   ╚═╝      ╚══════╝╚═╝  ╚═╝╚═╝     ╚═╝╚══════╝";
    let version = env!("CARGO_PKG_VERSION");
    let version_display = format!("{:^6}", version);

    let stops: [(u8, u8, u8); 3] = [
        (59, 130, 246), // Blue
        (6, 182, 212),  // Cyan
        (34, 197, 94),  // Green
    ];

    let mut banner_lines: Vec<Line> = Vec::new();
    for text in &lines {
        banner_lines.push(gradient_line(text, &stops));
    }

    // Line 5: gradient prefix + inverted version + gradient suffix
    let full_len =
        line5_prefix.chars().count() + version_display.len() + line5_suffix.chars().count();
    let mut line5_spans: Vec<Span> = Vec::new();
    for (i, ch) in line5_prefix.split_inclusive(|_: char| true).enumerate() {
        let t = i as f64 / (full_len - 1).max(1) as f64;
        let (r, g, b) = interpolate_stops(&stops, t);
        line5_spans.push(Span::styled(
            ch.to_string(),
            Style::default()
                .fg(Color::Rgb(r, g, b))
                .add_modifier(Modifier::BOLD),
        ));
    }
    // Version with inverted colors: colored background, black foreground
    let ver_pos = line5_prefix.chars().count();
    let ver_t = ver_pos as f64 / (full_len - 1).max(1) as f64;
    let (vr, vg, vb) = interpolate_stops(&stops, ver_t);
    line5_spans.push(Span::styled(
        version_display,
        Style::default()
            .fg(Color::White)
            .bg(Color::Rgb(vr, vg, vb))
            .add_modifier(Modifier::BOLD),
    ));
    let suffix_pos = ver_pos + 6;
    let t = suffix_pos as f64 / (full_len - 1).max(1) as f64;
    let (r, g, b) = interpolate_stops(&stops, t);
    line5_spans.push(Span::styled(
        line5_suffix.to_string(),
        Style::default()
            .fg(Color::Rgb(r, g, b))
            .add_modifier(Modifier::BOLD),
    ));
    banner_lines.push(Line::from(line5_spans));

    // Line 6: normal gradient
    banner_lines.push(gradient_line(last_line, &stops));

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
    (
        lerp(r1, r2, local_t),
        lerp(g1, g2, local_t),
        lerp(b1, b2, local_t),
    )
}

fn render_tagline(frame: &mut Frame, area: Rect) {
    let description = env!("CARGO_PKG_DESCRIPTION");

    let line = Line::from(Span::styled(
        description,
        Style::default()
            .fg(Color::DarkGray)
            .add_modifier(Modifier::BOLD),
    ));
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
            let last = ws
                .last_synced
                .as_deref()
                .map(format_timestamp)
                .unwrap_or_else(|| "never".to_string());
            let provider = ws.provider.kind.display_name();

            // Line 1: Path + provider
            let top = Line::from(vec![
                Span::styled("Path: ", dim),
                Span::styled(&ws.base_path, cyan),
                sep,
                Span::styled("Provider: ", dim),
                Span::styled(provider, cyan),
            ]);

            // Line 2: Synced sentence
            let synced_text = match &ws.last_synced {
                Some(_) => format!("Synced {} with {} {}", ws.base_path, provider, last),
                None => format!("{} with {} — never synced", ws.base_path, provider),
            };
            let bottom = Line::from(vec![Span::styled(synced_text, dim)]);

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
    let total_owners = app
        .local_repos
        .iter()
        .map(|r| r.owner.as_str())
        .collect::<HashSet<_>>()
        .len();
    let uncommitted = app.local_repos.iter().filter(|r| r.is_dirty).count();
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
        &total_owners.to_string(),
        "Owners",
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
        &clean.to_string(),
        "Clean",
        Color::Green,
        selected == 2,
    );
    render_stat_box(
        frame,
        cols[3],
        &behind.to_string(),
        "Behind",
        Color::Blue,
        selected == 3,
    );
    render_stat_box(
        frame,
        cols[4],
        &ahead.to_string(),
        "Ahead",
        Color::Blue,
        selected == 4,
    );
    render_stat_box(
        frame,
        cols[5],
        &uncommitted.to_string(),
        "Uncommitted",
        Color::Yellow,
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

fn render_tab_content(app: &App, frame: &mut Frame, area: Rect) {
    if area.height < 2 {
        return;
    }

    match app.stat_index {
        0 => render_owners_tab(app, frame, area),
        1 => render_repos_tab(app, frame, area),
        2 => render_clean_tab(app, frame, area),
        3 => render_behind_tab(app, frame, area),
        4 => render_ahead_tab(app, frame, area),
        5 => render_uncommitted_tab(app, frame, area),
        _ => {}
    }
}

fn render_owners_tab(app: &App, frame: &mut Frame, area: Rect) {
    let mut owner_stats: HashMap<&str, (usize, usize)> = HashMap::new();
    for r in &app.local_repos {
        let entry = owner_stats.entry(r.owner.as_str()).or_insert((0, 0));
        entry.0 += 1;
        if !r.is_dirty && r.behind == 0 && r.ahead == 0 {
            entry.1 += 1;
        }
    }

    let mut owners: Vec<(&str, usize, usize)> = owner_stats
        .into_iter()
        .map(|(name, (total, clean))| (name, total, clean))
        .collect();
    owners.sort_by_key(|(name, _, _)| name.to_lowercase());

    if owners.is_empty() {
        let msg = Paragraph::new(Line::from(Span::styled(
            "  No owners found. Run sync first.",
            Style::default().fg(Color::DarkGray),
        )))
        .block(
            Block::default()
                .title(" Owners ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        );
        frame.render_widget(msg, area);
        return;
    }

    let header = Row::new(vec!["Owner", "Repos", "Synced", "Needs Attention"])
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .bottom_margin(1);

    let rows: Vec<Row> = owners
        .iter()
        .enumerate()
        .map(|(i, (name, total, clean))| {
            let style = if i == app.dashboard_list_index {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            let needs_attention = total - clean;
            Row::new(vec![
                name.to_string(),
                total.to_string(),
                clean.to_string(),
                if needs_attention > 0 {
                    needs_attention.to_string()
                } else {
                    ".".to_string()
                },
            ])
            .style(style)
        })
        .collect();

    let widths = [
        Constraint::Percentage(40),
        Constraint::Percentage(15),
        Constraint::Percentage(20),
        Constraint::Percentage(25),
    ];

    let table = Table::new(rows, widths).header(header).block(
        Block::default()
            .title(" Owners ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan)),
    );
    frame.render_widget(table, area);
}

fn render_repos_tab(app: &App, frame: &mut Frame, area: Rect) {
    let repos: Vec<&RepoEntry> = app
        .local_repos
        .iter()
        .filter(|r| r.is_dirty || r.behind > 0 || r.ahead > 0)
        .collect();
    render_repo_table(app, frame, area, &repos, " Repos (needs attention) ");
}

fn render_clean_tab(app: &App, frame: &mut Frame, area: Rect) {
    let clean_count = app
        .local_repos
        .iter()
        .filter(|r| !r.is_dirty && r.behind == 0 && r.ahead == 0)
        .count();

    let msg = format!(
        "  {} repo{} clean — fully synced, no uncommitted changes.",
        clean_count,
        if clean_count == 1 { " is" } else { "s are" }
    );

    let content = Paragraph::new(Line::from(Span::styled(
        msg,
        Style::default().fg(Color::Green),
    )))
    .block(
        Block::default()
            .title(" Clean ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Green)),
    );
    frame.render_widget(content, area);
}

fn render_behind_tab(app: &App, frame: &mut Frame, area: Rect) {
    let repos: Vec<&RepoEntry> = app.local_repos.iter().filter(|r| r.behind > 0).collect();
    render_repo_table(app, frame, area, &repos, " Behind Remote ");
}

fn render_ahead_tab(app: &App, frame: &mut Frame, area: Rect) {
    let repos: Vec<&RepoEntry> = app.local_repos.iter().filter(|r| r.ahead > 0).collect();
    render_repo_table(app, frame, area, &repos, " Ahead of Remote ");
}

fn render_uncommitted_tab(app: &App, frame: &mut Frame, area: Rect) {
    let repos: Vec<&RepoEntry> = app.local_repos.iter().filter(|r| r.is_dirty).collect();
    render_repo_table(app, frame, area, &repos, " Uncommitted Changes ");
}

fn render_repo_table(app: &App, frame: &mut Frame, area: Rect, repos: &[&RepoEntry], title: &str) {
    if repos.is_empty() {
        let msg = Paragraph::new(Line::from(Span::styled(
            "  No repositories in this category.",
            Style::default().fg(Color::DarkGray),
        )))
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        );
        frame.render_widget(msg, area);
        return;
    }

    let header = Row::new(vec!["Org/Repo", "Branch", "Dirty", "Ahead", "Behind"])
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
            let style = if i == app.dashboard_list_index {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            let branch = entry.branch.as_deref().unwrap_or("-");
            let dirty = if entry.is_dirty { "*" } else { "." };
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
                dirty.to_string(),
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
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    frame.render_widget(table, area);
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
        Span::styled("[qq]", key_style),
        Span::styled(" Quit", dim),
        Span::raw("   "),
        Span::styled("[Esc]", key_style),
        Span::styled(" Back", dim),
        Span::raw("   "),
        Span::styled("[w]", key_style),
        Span::styled(" Workspace", dim),
    ];

    let right_spans = vec![
        Span::styled("[↑/↓]", key_style),
        Span::styled(" Up/Down", dim),
        Span::raw("   "),
        Span::styled("[←/→]", key_style),
        Span::styled(" Left/Right", dim),
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
