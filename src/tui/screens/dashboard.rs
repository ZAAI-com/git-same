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

use crossterm::event::{KeyCode, KeyEvent};
use tokio::sync::mpsc::UnboundedSender;

use crate::banner::{render_animated_banner, render_banner};
use crate::tui::app::{App, Operation, OperationState, Screen};
use crate::types::RepoEntry;
use crate::tui::event::AppEvent;

// ── Key handler ─────────────────────────────────────────────────────────────

pub async fn handle_key(app: &mut App, key: KeyEvent, backend_tx: &UnboundedSender<AppEvent>) {
    match key.code {
        KeyCode::Char('s') => {
            start_sync_operation(app, backend_tx);
        }
        KeyCode::Char('p') => {
            show_sync_progress(app);
        }
        KeyCode::Char('t') => {
            app.last_status_scan = None; // Force immediate refresh
            app.status_loading = true;
            start_operation(app, Operation::Status, backend_tx);
        }
        // Tab shortcuts
        KeyCode::Char('o') => {
            app.stat_index = 0;
            app.dashboard_table_state.select(Some(0));
        }
        KeyCode::Char('r') => {
            app.stat_index = 1;
            app.dashboard_table_state.select(Some(0));
        }
        KeyCode::Char('c') => {
            app.stat_index = 2;
            app.dashboard_table_state.select(Some(0));
        }
        KeyCode::Char('b') => {
            app.stat_index = 3;
            app.dashboard_table_state.select(Some(0));
        }
        KeyCode::Char('a') => {
            app.stat_index = 4;
            app.dashboard_table_state.select(Some(0));
        }
        KeyCode::Char('u') => {
            app.stat_index = 5;
            app.dashboard_table_state.select(Some(0));
        }
        KeyCode::Char('e') => {
            app.navigate_to(Screen::Settings);
        }
        KeyCode::Char('w') => {
            app.navigate_to(Screen::Workspaces);
        }
        KeyCode::Char('i') => {
            app.navigate_to(Screen::Settings);
        }
        KeyCode::Char('/') => {
            app.filter_active = true;
            app.filter_text.clear();
            app.stat_index = 1;
            app.dashboard_table_state.select(Some(0));
        }
        // Tab navigation (left/right between stat boxes)
        KeyCode::Left => {
            app.stat_index = app.stat_index.saturating_sub(1);
            app.dashboard_table_state.select(Some(0));
        }
        KeyCode::Right if app.stat_index < 5 => {
            app.stat_index += 1;
            app.dashboard_table_state.select(Some(0));
        }
        // List navigation (up/down within tab content)
        KeyCode::Down => {
            let count = tab_item_count(app);
            if count > 0 {
                let current = app.dashboard_table_state.selected().unwrap_or(0);
                if current + 1 < count {
                    app.dashboard_table_state.select(Some(current + 1));
                }
            }
        }
        KeyCode::Up => {
            let count = tab_item_count(app);
            if count > 0 {
                let current = app.dashboard_table_state.selected().unwrap_or(0);
                app.dashboard_table_state
                    .select(Some(current.saturating_sub(1)));
            }
        }
        KeyCode::Enter => {
            // Open the selected repo's folder
            if let Some(path) = selected_repo_path(app) {
                let _ = std::process::Command::new("open").arg(&path).spawn();
            }
        }
        _ => {}
    }
}

fn start_operation(app: &mut App, operation: Operation, backend_tx: &UnboundedSender<AppEvent>) {
    if matches!(
        app.operation_state,
        OperationState::Discovering { .. } | OperationState::Running { .. }
    ) {
        app.error_message = Some("An operation is already running".to_string());
        return;
    }

    app.tick_count = 0;
    app.operation_state = OperationState::Discovering {
        operation,
        message: format!("Starting {}...", operation),
    };
    app.log_lines.clear();
    app.scroll_offset = 0;

    crate::tui::backend::spawn_operation(operation, app, backend_tx.clone());
}

pub(crate) fn start_sync_operation(app: &mut App, backend_tx: &UnboundedSender<AppEvent>) {
    start_operation(app, Operation::Sync, backend_tx);
}

pub(crate) fn show_sync_progress(app: &mut App) {
    if !matches!(app.screen, Screen::Sync) {
        app.screen_stack.push(app.screen);
        app.screen = Screen::Sync;
    }
}

pub(crate) fn hide_sync_progress(app: &mut App) {
    if !matches!(app.screen, Screen::Sync) {
        return;
    }

    if app.screen_stack.is_empty() {
        app.screen = Screen::Dashboard;
    } else {
        app.go_back();
    }
}

fn tab_item_count(app: &App) -> usize {
    match app.stat_index {
        0 => app
            .local_repos
            .iter()
            .map(|r| r.owner.as_str())
            .collect::<HashSet<_>>()
            .len(),
        1 => {
            if app.filter_text.is_empty() {
                app.local_repos.len()
            } else {
                let ft = app.filter_text.to_lowercase();
                app.local_repos
                    .iter()
                    .filter(|r| r.full_name.to_lowercase().contains(&ft))
                    .count()
            }
        }
        2 => app
            .local_repos
            .iter()
            .filter(|r| !r.is_uncommitted && r.behind == 0 && r.ahead == 0)
            .count(),
        3 => app.local_repos.iter().filter(|r| r.behind > 0).count(),
        4 => app.local_repos.iter().filter(|r| r.ahead > 0).count(),
        5 => app.local_repos.iter().filter(|r| r.is_uncommitted).count(),
        _ => 0,
    }
}

fn selected_repo_path(app: &App) -> Option<std::path::PathBuf> {
    let selected = app.dashboard_table_state.selected()?;
    let repos: Vec<&RepoEntry> = match app.stat_index {
        0 => return None, // Owners tab — no single repo
        1 => {
            if app.filter_text.is_empty() {
                app.local_repos.iter().collect()
            } else {
                let ft = app.filter_text.to_lowercase();
                app.local_repos
                    .iter()
                    .filter(|r| r.full_name.to_lowercase().contains(&ft))
                    .collect()
            }
        }
        2 => app
            .local_repos
            .iter()
            .filter(|r| !r.is_uncommitted && r.behind == 0 && r.ahead == 0)
            .collect(),
        3 => app.local_repos.iter().filter(|r| r.behind > 0).collect(),
        4 => app.local_repos.iter().filter(|r| r.ahead > 0).collect(),
        5 => app
            .local_repos
            .iter()
            .filter(|r| r.is_uncommitted)
            .collect(),
        _ => return None,
    };
    repos.get(selected).map(|r| r.path.clone())
}

// ── Render ──────────────────────────────────────────────────────────────────

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

fn sync_banner_phase(app: &App) -> Option<f64> {
    match &app.operation_state {
        OperationState::Discovering {
            operation: Operation::Sync,
            ..
        }
        | OperationState::Running {
            operation: Operation::Sync,
            ..
        } => Some((app.tick_count as f64 / 50.0).fract()),
        _ => None,
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

    if let Some(phase) = sync_banner_phase(app) {
        render_animated_banner(frame, chunks[0], phase);
    } else {
        render_banner(frame, chunks[0]);
    }
    render_tagline(frame, chunks[1]);
    render_config_reqs(app, frame, chunks[2]);
    render_workspace_info(app, frame, chunks[3]);
    let stat_cols = render_stats(app, frame, chunks[4]);
    let table_area = Rect {
        y: chunks[5].y + 1,
        height: chunks[5].height.saturating_sub(1),
        ..chunks[5]
    };
    render_tab_content(app, frame, table_area);
    render_tab_connector(frame, &stat_cols, chunks[5], app.stat_index);
    render_bottom_actions(app, frame, chunks[6]);
}

fn render_tagline(frame: &mut Frame, area: Rect) {
    let description = crate::banner::subheadline();

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
    let cols =
        Layout::horizontal([Constraint::Percentage(41), Constraint::Percentage(59)]).split(area);
    frame.render_widget(Paragraph::new(Line::from(left)).right_aligned(), cols[0]);
    frame.render_widget(Paragraph::new(Line::from(right)), cols[1]);
}

fn render_config_reqs(app: &App, frame: &mut Frame, area: Rect) {
    let dim = Style::default().fg(Color::DarkGray);

    let key_style = Style::default()
        .fg(Color::Rgb(37, 99, 235))
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
                Span::styled(" [✓]", Style::default().fg(Color::Rgb(21, 128, 61))),
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
    let key_style = Style::default()
        .fg(Color::Rgb(37, 99, 235))
        .add_modifier(Modifier::BOLD);
    match &app.active_workspace {
        Some(ws) => {
            let folder_name = ws
                .root_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_else(|| ws.root_path.to_str().unwrap_or("workspace"))
                .to_string();

            render_info_line(
                frame,
                area,
                vec![
                    Span::styled("[w]", key_style),
                    Span::styled(" Workspace   ", dim),
                ],
                vec![
                    Span::styled(" [✓]", Style::default().fg(Color::Rgb(21, 128, 61))),
                    Span::styled(" Folder ", dim),
                    Span::styled(
                        folder_name,
                        Style::default()
                            .fg(Color::Rgb(21, 128, 61))
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw("  "),
                    Span::styled("[/]", key_style),
                    Span::styled(" Search Repositories", dim),
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

    let completed_repos = app.local_repos.len();
    let completed_owners = app
        .local_repos
        .iter()
        .map(|r| r.owner.as_str())
        .collect::<HashSet<_>>()
        .len();
    let discovered_repos = app.all_repos.len();
    let discovered_owners = app
        .all_repos
        .iter()
        .map(|r| r.owner.as_str())
        .collect::<HashSet<_>>()
        .len();
    let total_repos = discovered_repos.max(completed_repos);
    let total_owners = discovered_owners.max(completed_owners);
    let owners_progress = format!("{}/{}", completed_owners, total_owners);
    let repos_progress = total_repos.to_string();
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
        &owners_progress,
        "o",
        "Owners",
        Color::Rgb(21, 128, 61),
        selected == 0,
    );
    render_stat_box(
        frame,
        cols[1],
        &repos_progress,
        "r",
        "Repositories",
        Color::Rgb(21, 128, 61),
        selected == 1,
    );
    render_stat_box(
        frame,
        cols[2],
        &clean.to_string(),
        "c",
        "Clean",
        Color::Rgb(21, 128, 61),
        selected == 2,
    );
    render_stat_box(
        frame,
        cols[3],
        &behind.to_string(),
        "b",
        "Behind",
        Color::Rgb(21, 128, 61),
        selected == 3,
    );
    render_stat_box(
        frame,
        cols[4],
        &ahead.to_string(),
        "a",
        "Ahead",
        Color::Rgb(21, 128, 61),
        selected == 4,
    );
    render_stat_box(
        frame,
        cols[5],
        &uncommitted.to_string(),
        "u",
        "Uncommitted",
        Color::Rgb(21, 128, 61),
        selected == 5,
    );

    [cols[0], cols[1], cols[2], cols[3], cols[4], cols[5]]
}

fn render_stat_box(
    frame: &mut Frame,
    area: Rect,
    value: &str,
    key: &str,
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
        Line::from(vec![
            Span::styled(
                format!("[{}]", key),
                Style::default()
                    .fg(Color::Rgb(37, 99, 235))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
            Span::styled(label, Style::default().fg(Color::DarkGray)),
        ]),
    ])
    .centered()
    .block(block);
    frame.render_widget(content, area);
}

fn tab_color(_stat_index: usize) -> Color {
    Color::Rgb(21, 128, 61)
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
                format!("{:>4}", i + 1),
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
                format!("{:>4}", i + 1),
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
                format!("{:>4}", i + 1),
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
                format!("{:>4}", i + 1),
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
                format!("{:>4}", i + 1),
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
                format!("{:>4}", i + 1),
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
            .fg(Color::Rgb(21, 128, 61))
            .add_modifier(Modifier::BOLD),
    )
    .bottom_margin(1);

    let table = Table::new(rows, widths)
        .header(header)
        .row_highlight_style(
            Style::default()
                .fg(Color::Rgb(21, 128, 61))
                .add_modifier(Modifier::BOLD),
        )
        .block(block);
    frame.render_stateful_widget(table, area, table_state);
}

fn render_bottom_actions(app: &App, frame: &mut Frame, area: Rect) {
    let rows = Layout::vertical([
        Constraint::Length(1), // Actions + sync info
        Constraint::Length(1), // Navigation
    ])
    .split(area);

    let dim = Style::default().fg(Color::DarkGray);
    let key_style = Style::default()
        .fg(Color::Rgb(37, 99, 235))
        .add_modifier(Modifier::BOLD);

    // Line 1: live sync status (centered full-width) + action hints (right overlay)
    let sync_line = match &app.operation_state {
        OperationState::Discovering {
            operation: Operation::Sync,
            message,
        } => Some(Line::from(vec![
            Span::styled(
                "Sync ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("discovering in background", dim),
            Span::styled(": ", dim),
            Span::styled(message.clone(), dim),
        ])),
        OperationState::Running {
            operation: Operation::Sync,
            completed,
            total,
            started_at,
            throughput_samples,
            active_repos,
            ..
        } => {
            let pct = if *total > 0 {
                ((*completed as f64 / *total as f64) * 100.0).round() as u64
            } else {
                0
            };
            let elapsed_secs = started_at.elapsed().as_secs_f64();
            let sample_count = throughput_samples.len().min(10);
            let sample_rate = if sample_count > 0 {
                throughput_samples
                    .iter()
                    .rev()
                    .take(sample_count)
                    .copied()
                    .sum::<u64>() as f64
                    / sample_count as f64
            } else {
                0.0
            };
            let repos_per_sec = if sample_rate > 0.0 {
                sample_rate
            } else if elapsed_secs > 1.0 {
                *completed as f64 / elapsed_secs
            } else {
                0.0
            };
            let remaining = total.saturating_sub(*completed);
            let has_eta_data = throughput_samples.iter().any(|&n| n > 0);
            let eta_secs = if has_eta_data && repos_per_sec > 0.1 {
                Some((remaining as f64 / repos_per_sec).ceil() as u64)
            } else {
                None
            };
            let concurrency = app
                .active_workspace
                .as_ref()
                .and_then(|ws| ws.concurrency)
                .unwrap_or(app.config.concurrency);

            let mut spans = vec![
                Span::styled(
                    "Sync ",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled("running in background ", dim),
                Span::styled(format!("{}%", pct), Style::default().fg(Color::Cyan)),
                Span::styled(format!(" ({}/{})", completed, total), dim),
            ];

            if repos_per_sec > 0.0 {
                spans.push(Span::styled(
                    format!("  |  {:.1} repo/s", repos_per_sec),
                    Style::default().fg(Color::DarkGray),
                ));
            }
            if let Some(eta_secs) = eta_secs.filter(|_| remaining > 0) {
                spans.push(Span::styled(
                    format!("  |  ETA {}", format_duration_secs(eta_secs)),
                    Style::default().fg(Color::Cyan),
                ));
            }
            spans.push(Span::styled(
                format!("  |  workers {}/{}", active_repos.len(), concurrency),
                Style::default().fg(Color::DarkGray),
            ));
            spans.push(Span::styled("  |  show ", dim));
            spans.push(Span::styled("[p]", key_style));
            spans.push(Span::styled(" progress", dim));
            Some(Line::from(spans))
        }
        OperationState::Finished {
            operation: Operation::Sync,
            summary,
            with_updates,
            duration_secs,
            ..
        } => {
            let total = summary.success + summary.failed + summary.skipped;
            Some(Line::from(vec![
                Span::styled(
                    "Last Sync ",
                    Style::default()
                        .fg(Color::Rgb(21, 128, 61))
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("{} repos", total),
                    Style::default().fg(Color::Rgb(21, 128, 61)),
                ),
                Span::styled(
                    format!("  |  {} updated", with_updates),
                    Style::default().fg(Color::Yellow),
                ),
                Span::styled(
                    format!("  |  {} failed", summary.failed),
                    if summary.failed > 0 {
                        Style::default().fg(Color::Red)
                    } else {
                        Style::default().fg(Color::DarkGray)
                    },
                ),
                Span::styled(
                    format!("  |  {:.1}s", duration_secs),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled("  |  details ", dim),
                Span::styled("[p]", key_style),
            ]))
        }
        _ => app.active_workspace.as_ref().and_then(|ws| {
            ws.last_synced.as_ref().map(|ts| {
                let folder_name_owned = ws
                    .root_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or_else(|| ws.root_path.to_str().unwrap_or("workspace"))
                    .to_string();
                let folder_name = folder_name_owned.as_str();
                let formatted = format_timestamp(ts);
                Line::from(vec![
                    Span::styled("Synced ", dim),
                    Span::styled(
                        folder_name.to_string(),
                        Style::default()
                            .fg(Color::Rgb(21, 128, 61))
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(" with GitHub ", dim),
                    Span::styled(formatted, dim),
                ])
            })
        }),
    };
    if let Some(sync_line) = sync_line {
        frame.render_widget(Paragraph::new(vec![sync_line]).centered(), rows[0]);
    }

    let actions_right = Line::from(vec![
        Span::styled("[s]", key_style),
        Span::styled(" Start Sync", dim),
        Span::raw("   "),
        Span::styled("[p]", key_style),
        Span::styled(" Show Sync Progress", dim),
        Span::raw(" "),
    ]);
    frame.render_widget(Paragraph::new(vec![actions_right]).right_aligned(), rows[0]);

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

    let nav_left = Paragraph::new(vec![Line::from(left_spans)]);
    let nav_right = Paragraph::new(vec![Line::from(right_spans)]).right_aligned();

    frame.render_widget(nav_left, nav_cols[0]);
    frame.render_widget(nav_right, nav_cols[1]);
}

fn format_duration_secs(secs: u64) -> String {
    if secs >= 60 {
        format!("{}m{}s", secs / 60, secs % 60)
    } else {
        format!("{}s", secs)
    }
}

#[cfg(test)]
#[path = "dashboard_tests.rs"]
mod tests;
