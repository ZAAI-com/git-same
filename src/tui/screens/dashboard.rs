//! Dashboard screen — home view with summary stats and quick-action hotkeys.

use std::collections::{HashMap, HashSet};

use ratatui::{
    layout::{Constraint, Layout, Position, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph, Row, Table, TableState},
    Frame,
};

use chrono::DateTime;

use crate::banner::render_banner;
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

pub fn render(app: &mut App, frame: &mut Frame) {
    let chunks = Layout::vertical([
        Constraint::Length(6), // Banner
        Constraint::Length(1), // Tagline
        Constraint::Length(1), // Requirements
        Constraint::Length(1), // Workspace
        Constraint::Length(4), // Stats
        Constraint::Min(1),    // Spacer
        Constraint::Length(2), // Bottom actions
    ])
    .split(frame.area());

    render_banner(frame, chunks[0]);
    render_tagline(frame, chunks[1]);
    render_config_reqs(app, frame, chunks[2]);
    render_workspace_info(app, frame, chunks[3]);
    let stat_cols = render_stats(app, frame, chunks[4]);
    render_tab_content(app, frame, chunks[5]);
    render_tab_connector(frame, &stat_cols, chunks[5], app.stat_index);
    render_bottom_actions(app, frame, chunks[6]);
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

fn render_info_line(frame: &mut Frame, area: Rect, left: Vec<Span>, right: Vec<Span>) {
    let cols = Layout::horizontal([
        Constraint::Percentage(50),
        Constraint::Percentage(50),
    ])
    .split(area);
    frame.render_widget(Paragraph::new(Line::from(left)).right_aligned(), cols[0]);
    frame.render_widget(Paragraph::new(Line::from(right)), cols[1]);
}

fn render_config_reqs(app: &App, frame: &mut Frame, area: Rect) {
    let dim = Style::default().fg(Color::DarkGray);

    let key_style = Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD);
    let left = vec![
        Span::styled("[e]", key_style),
        Span::styled(" Settings    ", dim),
    ];

    let right = if app.checks_loading || app.check_results.is_empty() {
        vec![
            Span::styled(" Checking...", Style::default().fg(Color::Yellow)),
            Span::raw("  "),
            Span::styled("[t]", key_style),
            Span::styled(" Refresh", dim),
        ]
    } else {
        let all_passed = app.check_results.iter().all(|c| c.passed);
        if all_passed {
            vec![
                Span::styled(" [✓]", Style::default().fg(Color::Green)),
                Span::styled(" Requirements Satisfied", dim),
                Span::raw("  "),
                Span::styled("[t]", key_style),
                Span::styled(" Refresh", dim),
            ]
        } else {
            vec![
                Span::styled(" [✗]", Style::default().fg(Color::Red)),
                Span::styled(" Requirements Not Met", dim),
                Span::raw("  "),
                Span::styled("[t]", key_style),
                Span::styled(" Refresh", dim),
            ]
        }
    };

    render_info_line(frame, area, left, right);
}

fn render_workspace_info(app: &App, frame: &mut Frame, area: Rect) {
    let dim = Style::default().fg(Color::DarkGray);
    let cyan = Style::default().fg(Color::Cyan);
    let key_style = Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD);
    match &app.active_workspace {
        Some(ws) => {
            let folder_name = std::path::Path::new(&ws.base_path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(&ws.base_path)
                .to_string();

            render_info_line(
                frame,
                area,
                vec![
                    Span::styled(
                        "[w]",
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(" Workspace   ", dim),
                ],
                vec![
                    Span::styled(" [✓]", Style::default().fg(Color::Green)),
                    Span::styled(" Folder ", dim),
                    Span::styled(
                        folder_name,
                        cyan.add_modifier(Modifier::ITALIC | Modifier::BOLD),
                    ),
                    Span::raw("  "),
                    Span::styled("[/]", key_style),
                    Span::styled(" Search", dim),
                ],
            );
        }
        None => {
            let p = Paragraph::new(Line::from(Span::styled(
                "No workspace selected",
                Style::default().fg(Color::Yellow),
            )))
            .centered();
            frame.render_widget(p, area);
        }
    }
}

fn render_stats(app: &App, frame: &mut Frame, area: Rect) -> [Rect; 6] {
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
    let uncommitted = app.local_repos.iter().filter(|r| r.is_uncommitted).count();
    let behind = app.local_repos.iter().filter(|r| r.behind > 0).count();
    let ahead = app.local_repos.iter().filter(|r| r.ahead > 0).count();
    let clean = app
        .local_repos
        .iter()
        .filter(|r| !r.is_uncommitted && r.behind == 0 && r.ahead == 0)
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
        "Repositories",
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

    [cols[0], cols[1], cols[2], cols[3], cols[4], cols[5]]
}

fn render_stat_box(
    frame: &mut Frame,
    area: Rect,
    value: &str,
    label: &str,
    color: Color,
    selected: bool,
) {
    let (border_style, borders, border_type) = if selected {
        (
            Style::default().fg(color).add_modifier(Modifier::BOLD),
            Borders::TOP | Borders::LEFT | Borders::RIGHT,
            BorderType::Thick,
        )
    } else {
        (
            Style::default().fg(Color::DarkGray),
            Borders::ALL,
            BorderType::Plain,
        )
    };
    let block = Block::default()
        .borders(borders)
        .border_type(border_type)
        .border_style(border_style);
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

fn tab_color(stat_index: usize) -> Color {
    match stat_index {
        0 => Color::Cyan,
        1 => Color::Cyan,
        2 => Color::Green,
        3 => Color::Blue,
        4 => Color::Blue,
        5 => Color::Yellow,
        _ => Color::DarkGray,
    }
}

fn render_tab_connector(
    frame: &mut Frame,
    stat_cols: &[Rect; 6],
    content_area: Rect,
    selected: usize,
) {
    let color = tab_color(selected);
    let style = Style::default().fg(color).add_modifier(Modifier::BOLD);
    let y = content_area.y;
    let x_start = content_area.x;
    let x_end = content_area.x + content_area.width.saturating_sub(1);
    let tab_left = stat_cols[selected].x;
    let tab_right = stat_cols[selected].x + stat_cols[selected].width.saturating_sub(1);

    let buf = frame.buffer_mut();

    for x in x_start..=x_end {
        let symbol = if (x == tab_left && x == x_start) || (x == tab_right && x == x_end) {
            "┃" // tab edge aligns with content edge: vertical continues
        } else if x == tab_left {
            "┛" // horizontal from left meets tab's left border going up
        } else if x == tab_right {
            "┗" // tab's right border going up meets horizontal going right
        } else if x > tab_left && x < tab_right {
            " " // gap under the selected tab
        } else if x == x_start {
            "┏" // content top-left corner
        } else if x == x_end {
            "┓" // content top-right corner
        } else {
            "━" // thick horizontal line
        };

        if let Some(cell) = buf.cell_mut(Position::new(x, y)) {
            cell.set_symbol(symbol);
            cell.set_style(style);
        }
    }
}

fn render_tab_content(app: &mut App, frame: &mut Frame, area: Rect) {
    if area.height < 2 {
        return;
    }

    let color = tab_color(app.stat_index);
    let mut table_state = app.dashboard_table_state;
    match app.stat_index {
        0 => render_owners_tab(app, frame, area, color, &mut table_state),
        1 => render_repos_tab(app, frame, area, color, &mut table_state),
        2 => render_clean_tab(app, frame, area, color, &mut table_state),
        3 => render_behind_tab(app, frame, area, color, &mut table_state),
        4 => render_ahead_tab(app, frame, area, color, &mut table_state),
        5 => render_uncommitted_tab(app, frame, area, color, &mut table_state),
        _ => {}
    }
    app.dashboard_table_state = table_state;
}

fn render_owners_tab(
    app: &App,
    frame: &mut Frame,
    area: Rect,
    color: Color,
    table_state: &mut TableState,
) {
    // (total, behind, ahead, uncommitted)
    let mut owner_stats: HashMap<&str, (usize, usize, usize, usize)> = HashMap::new();
    for r in &app.local_repos {
        let entry = owner_stats.entry(r.owner.as_str()).or_insert((0, 0, 0, 0));
        entry.0 += 1;
        if r.behind > 0 {
            entry.1 += 1;
        }
        if r.ahead > 0 {
            entry.2 += 1;
        }
        if r.is_uncommitted {
            entry.3 += 1;
        }
    }

    let mut owners: Vec<(&str, usize, usize, usize, usize)> = owner_stats
        .into_iter()
        .map(|(name, (total, behind, ahead, uncommitted))| {
            (name, total, behind, ahead, uncommitted)
        })
        .collect();
    owners.sort_by_key(|(name, _, _, _, _)| name.to_lowercase());

    let header_cols = vec!["#", "Owner", "Repos", "Behind", "Ahead", "Uncommitted"];
    let widths = [
        Constraint::Length(4),
        Constraint::Percentage(35),
        Constraint::Percentage(15),
        Constraint::Percentage(15),
        Constraint::Percentage(15),
        Constraint::Percentage(20),
    ];

    let rows: Vec<Row> = owners
        .iter()
        .enumerate()
        .map(|(i, (name, total, behind, ahead, uncommitted))| {
            let fmt = |n: &usize| {
                if *n > 0 {
                    n.to_string()
                } else {
                    ".".to_string()
                }
            };
            Row::new(vec![
                (i + 1).to_string(),
                name.to_string(),
                total.to_string(),
                fmt(behind),
                fmt(ahead),
                fmt(uncommitted),
            ])
        })
        .collect();

    render_table_block(frame, area, &header_cols, rows, &widths, color, table_state);
}

fn render_repos_tab(
    app: &App,
    frame: &mut Frame,
    area: Rect,
    color: Color,
    table_state: &mut TableState,
) {
    let repos: Vec<&RepoEntry> = if app.filter_text.is_empty() {
        app.local_repos.iter().collect()
    } else {
        let ft = app.filter_text.to_lowercase();
        app.local_repos
            .iter()
            .filter(|r| r.full_name.to_lowercase().contains(&ft))
            .collect()
    };

    let header_cols = vec!["#", "Org/Repo", "Branch", "Uncommitted", "Ahead", "Behind"];
    let widths = [
        Constraint::Length(4),
        Constraint::Percentage(35),
        Constraint::Percentage(20),
        Constraint::Percentage(15),
        Constraint::Percentage(15),
        Constraint::Percentage(15),
    ];

    let rows: Vec<Row> = repos
        .iter()
        .enumerate()
        .map(|(i, entry)| {
            let branch = entry.branch.as_deref().unwrap_or("-");
            Row::new(vec![
                (i + 1).to_string(),
                entry.full_name.clone(),
                branch.to_string(),
                fmt_flag(entry.is_uncommitted),
                fmt_count_plus(entry.ahead),
                fmt_count_minus(entry.behind),
            ])
        })
        .collect();

    render_table_block(frame, area, &header_cols, rows, &widths, color, table_state);
}

fn render_clean_tab(
    app: &App,
    frame: &mut Frame,
    area: Rect,
    color: Color,
    table_state: &mut TableState,
) {
    let repos: Vec<&RepoEntry> = app
        .local_repos
        .iter()
        .filter(|r| !r.is_uncommitted && r.behind == 0 && r.ahead == 0)
        .collect();

    let header_cols = vec!["#", "Org/Repo", "Branch"];
    let widths = [
        Constraint::Length(4),
        Constraint::Percentage(60),
        Constraint::Percentage(40),
    ];

    let rows: Vec<Row> = repos
        .iter()
        .enumerate()
        .map(|(i, entry)| {
            let branch = entry.branch.as_deref().unwrap_or("-");
            Row::new(vec![
                (i + 1).to_string(),
                entry.full_name.clone(),
                branch.to_string(),
            ])
        })
        .collect();

    render_table_block(frame, area, &header_cols, rows, &widths, color, table_state);
}

fn render_behind_tab(
    app: &App,
    frame: &mut Frame,
    area: Rect,
    color: Color,
    table_state: &mut TableState,
) {
    let repos: Vec<&RepoEntry> = app.local_repos.iter().filter(|r| r.behind > 0).collect();

    let header_cols = vec!["#", "Org/Repo", "Branch", "Behind"];
    let widths = [
        Constraint::Length(4),
        Constraint::Percentage(45),
        Constraint::Percentage(30),
        Constraint::Percentage(25),
    ];

    let rows: Vec<Row> = repos
        .iter()
        .enumerate()
        .map(|(i, entry)| {
            let branch = entry.branch.as_deref().unwrap_or("-");
            Row::new(vec![
                (i + 1).to_string(),
                entry.full_name.clone(),
                branch.to_string(),
                fmt_count_minus(entry.behind),
            ])
        })
        .collect();

    render_table_block(frame, area, &header_cols, rows, &widths, color, table_state);
}

fn render_ahead_tab(
    app: &App,
    frame: &mut Frame,
    area: Rect,
    color: Color,
    table_state: &mut TableState,
) {
    let repos: Vec<&RepoEntry> = app.local_repos.iter().filter(|r| r.ahead > 0).collect();

    let header_cols = vec!["#", "Org/Repo", "Branch", "Ahead"];
    let widths = [
        Constraint::Length(4),
        Constraint::Percentage(45),
        Constraint::Percentage(30),
        Constraint::Percentage(25),
    ];

    let rows: Vec<Row> = repos
        .iter()
        .enumerate()
        .map(|(i, entry)| {
            let branch = entry.branch.as_deref().unwrap_or("-");
            Row::new(vec![
                (i + 1).to_string(),
                entry.full_name.clone(),
                branch.to_string(),
                fmt_count_plus(entry.ahead),
            ])
        })
        .collect();

    render_table_block(frame, area, &header_cols, rows, &widths, color, table_state);
}

fn render_uncommitted_tab(
    app: &App,
    frame: &mut Frame,
    area: Rect,
    color: Color,
    table_state: &mut TableState,
) {
    let repos: Vec<&RepoEntry> = app
        .local_repos
        .iter()
        .filter(|r| r.is_uncommitted)
        .collect();

    let header_cols = vec!["#", "Org/Repo", "Branch", "Staged", "Unstaged", "Untracked"];
    let widths = [
        Constraint::Length(4),
        Constraint::Percentage(30),
        Constraint::Percentage(22),
        Constraint::Percentage(16),
        Constraint::Percentage(16),
        Constraint::Percentage(16),
    ];

    let rows: Vec<Row> = repos
        .iter()
        .enumerate()
        .map(|(i, entry)| {
            let branch = entry.branch.as_deref().unwrap_or("-");
            let fmt_n = |n: usize| {
                if n > 0 {
                    n.to_string()
                } else {
                    ".".to_string()
                }
            };
            Row::new(vec![
                (i + 1).to_string(),
                entry.full_name.clone(),
                branch.to_string(),
                fmt_n(entry.staged_count),
                fmt_n(entry.unstaged_count),
                fmt_n(entry.untracked_count),
            ])
        })
        .collect();

    render_table_block(frame, area, &header_cols, rows, &widths, color, table_state);
}

// -- Shared helpers --

fn fmt_flag(flag: bool) -> String {
    if flag {
        "*".to_string()
    } else {
        ".".to_string()
    }
}

fn fmt_count_plus(n: usize) -> String {
    if n > 0 {
        format!("+{}", n)
    } else {
        ".".to_string()
    }
}

fn fmt_count_minus(n: usize) -> String {
    if n > 0 {
        format!("-{}", n)
    } else {
        ".".to_string()
    }
}

fn render_table_block(
    frame: &mut Frame,
    area: Rect,
    header_cols: &[&str],
    rows: Vec<Row>,
    widths: &[Constraint],
    color: Color,
    table_state: &mut TableState,
) {
    let border_style = Style::default().fg(color).add_modifier(Modifier::BOLD);
    let block = Block::default()
        .borders(Borders::LEFT | Borders::RIGHT | Borders::BOTTOM)
        .border_type(BorderType::Thick)
        .border_style(border_style);

    if rows.is_empty() {
        let msg = Paragraph::new(Line::from(Span::styled(
            "  No repositories in this category.",
            Style::default().fg(Color::DarkGray),
        )))
        .block(block);
        frame.render_widget(msg, area);
        return;
    }

    let header = Row::new(
        header_cols
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>(),
    )
    .style(
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )
    .bottom_margin(1);

    let table = Table::new(rows, widths)
        .header(header)
        .row_highlight_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .block(block);
    frame.render_stateful_widget(table, area, table_state);
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
    ];

    let right_spans = vec![
        Span::styled("[←]", key_style),
        Span::raw(" "),
        Span::styled("[↑]", key_style),
        Span::raw(" "),
        Span::styled("[↓]", key_style),
        Span::raw(" "),
        Span::styled("[→]", key_style),
        Span::styled(" Move", dim),
        Span::raw("   "),
        Span::styled("[Enter]", key_style),
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
