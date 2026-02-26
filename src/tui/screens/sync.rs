//! Sync progress screen — real-time metrics during sync, enriched summary after.

use ratatui::{
    layout::{Alignment, Constraint, Layout, Position, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Gauge, List, ListItem, Paragraph},
    Frame,
};

use crossterm::event::{KeyCode, KeyEvent};
use tokio::sync::mpsc::UnboundedSender;

use crate::tui::app::{App, LogFilter, OperationState, SyncLogEntry, SyncLogStatus};
use crate::tui::event::AppEvent;
use crate::tui::screens::dashboard::{hide_sync_progress, start_sync_operation};

use crate::banner::render_animated_banner;

// ── Key handler ─────────────────────────────────────────────────────────────

pub fn handle_key(app: &mut App, key: KeyEvent, backend_tx: &UnboundedSender<AppEvent>) {
    let is_finished = matches!(app.operation_state, OperationState::Finished { .. });

    match key.code {
        KeyCode::Char('s') => {
            start_sync_operation(app, backend_tx);
        }
        KeyCode::Char('p') => {
            hide_sync_progress(app);
        }
        // Scroll log
        KeyCode::Down => {
            if is_finished {
                if app.log_filter == LogFilter::Changelog {
                    app.changelog_scroll += 1;
                } else {
                    let count = filtered_log_count(app);
                    if count > 0 && app.sync_log_index < count.saturating_sub(1) {
                        app.sync_log_index += 1;
                    }
                }
            } else if app.scroll_offset < app.log_lines.len().saturating_sub(1) {
                app.scroll_offset += 1;
            }
        }
        KeyCode::Up => {
            if is_finished {
                if app.log_filter == LogFilter::Changelog {
                    app.changelog_scroll = app.changelog_scroll.saturating_sub(1);
                } else {
                    app.sync_log_index = app.sync_log_index.saturating_sub(1);
                }
            } else {
                app.scroll_offset = app.scroll_offset.saturating_sub(1);
            }
        }
        KeyCode::Left => {
            if is_finished {
                cycle_filter(app, backend_tx, -1);
            } else {
                app.scroll_offset = app.scroll_offset.saturating_sub(1);
            }
        }
        KeyCode::Right => {
            if is_finished {
                cycle_filter(app, backend_tx, 1);
            } else if app.scroll_offset < app.log_lines.len().saturating_sub(1) {
                app.scroll_offset += 1;
            }
        }
        // Expand/collapse commit deep dive
        KeyCode::Enter if is_finished => {
            // Extract data we need before mutating app
            let selected = filtered_log_entries(app)
                .get(app.sync_log_index)
                .map(|e| (e.repo_name.clone(), e.path.clone()));

            if let Some((repo_name, path)) = selected {
                if app.expanded_repo.as_deref() == Some(&repo_name) {
                    // Toggle off: collapse
                    app.expanded_repo = None;
                    app.repo_commits.clear();
                } else if let Some(path) = path {
                    // Expand: fetch commits
                    app.expanded_repo = Some(repo_name.clone());
                    app.repo_commits.clear();
                    crate::tui::backend::spawn_commit_fetch(path, repo_name, backend_tx.clone());
                }
            }
        }
        // Post-sync log filters
        KeyCode::Char('a') if is_finished => {
            apply_log_filter(app, backend_tx, LogFilter::All);
        }
        KeyCode::Char('u') if is_finished => {
            apply_log_filter(app, backend_tx, LogFilter::Updated);
        }
        KeyCode::Char('f') if is_finished => {
            apply_log_filter(app, backend_tx, LogFilter::Failed);
        }
        KeyCode::Char('x') if is_finished => {
            apply_log_filter(app, backend_tx, LogFilter::Skipped);
        }
        KeyCode::Char('c') if is_finished => {
            apply_log_filter(app, backend_tx, LogFilter::Changelog);
        }
        // Sync history overlay toggle
        KeyCode::Char('h') if is_finished => {
            app.show_sync_history = !app.show_sync_history;
        }
        _ => {}
    }
}

fn apply_log_filter(app: &mut App, backend_tx: &UnboundedSender<AppEvent>, filter: LogFilter) {
    app.log_filter = filter;
    app.sync_log_index = 0;
    app.expanded_repo = None;
    app.repo_commits.clear();
    app.changelog_scroll = 0;

    if filter != LogFilter::Changelog {
        return;
    }

    // Collect updated repos with paths for batch commit fetch.
    let updated_repos: Vec<(String, std::path::PathBuf)> = app
        .sync_log_entries
        .iter()
        .filter(|e| e.had_updates)
        .filter_map(|e| e.path.clone().map(|p| (e.repo_name.clone(), p)))
        .collect();
    app.changelog_total = updated_repos.len();
    app.changelog_loaded = 0;
    app.changelog_commits.clear();

    if !updated_repos.is_empty() {
        crate::tui::backend::spawn_changelog_fetch(updated_repos, backend_tx.clone());
    }
}

fn cycle_filter(app: &mut App, backend_tx: &UnboundedSender<AppEvent>, direction: i8) {
    const FILTERS: [LogFilter; 5] = [
        LogFilter::All,
        LogFilter::Updated,
        LogFilter::Failed,
        LogFilter::Skipped,
        LogFilter::Changelog,
    ];

    let idx = FILTERS
        .iter()
        .position(|f| *f == app.log_filter)
        .unwrap_or(0) as i8;
    let next = (idx + direction).rem_euclid(FILTERS.len() as i8) as usize;
    apply_log_filter(app, backend_tx, FILTERS[next]);
}

/// Count of log entries matching the current filter.
fn filtered_log_count(app: &App) -> usize {
    match app.log_filter {
        LogFilter::All => app.sync_log_entries.len(),
        LogFilter::Updated => app
            .sync_log_entries
            .iter()
            .filter(|e| e.had_updates || e.is_clone)
            .count(),
        LogFilter::Failed => app
            .sync_log_entries
            .iter()
            .filter(|e| e.status == SyncLogStatus::Failed)
            .count(),
        LogFilter::Skipped => app
            .sync_log_entries
            .iter()
            .filter(|e| e.status == SyncLogStatus::Skipped)
            .count(),
        LogFilter::Changelog => app
            .sync_log_entries
            .iter()
            .filter(|e| e.had_updates)
            .count(),
    }
}

/// Returns filtered log entries matching the current filter.
fn filtered_log_entries(app: &App) -> Vec<&SyncLogEntry> {
    match app.log_filter {
        LogFilter::All => app.sync_log_entries.iter().collect(),
        LogFilter::Updated => app
            .sync_log_entries
            .iter()
            .filter(|e| e.had_updates || e.is_clone)
            .collect(),
        LogFilter::Failed => app
            .sync_log_entries
            .iter()
            .filter(|e| e.status == SyncLogStatus::Failed)
            .collect(),
        LogFilter::Skipped => app
            .sync_log_entries
            .iter()
            .filter(|e| e.status == SyncLogStatus::Skipped)
            .collect(),
        LogFilter::Changelog => app
            .sync_log_entries
            .iter()
            .filter(|e| e.had_updates)
            .collect(),
    }
}

// ── Render ──────────────────────────────────────────────────────────────────

const POPUP_WIDTH_PERCENT: u16 = 80;
const POPUP_HEIGHT_PERCENT: u16 = 80;

pub fn render(app: &App, frame: &mut Frame) {
    let is_finished = matches!(&app.operation_state, OperationState::Finished { .. });

    // Animate during active ops, static otherwise
    let phase = match &app.operation_state {
        OperationState::Discovering { .. } | OperationState::Running { .. } => {
            (app.tick_count as f64 / 50.0).fract()
        }
        _ => 0.0,
    };

    let popup_area = centered_rect(frame.area(), POPUP_WIDTH_PERCENT, POPUP_HEIGHT_PERCENT);
    dim_outside_popup(frame, popup_area);
    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(" Sync Progress ")
        .borders(Borders::ALL)
        .border_type(BorderType::Thick)
        .border_style(Style::default().fg(Color::Cyan));
    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    render_running_layout(app, frame, inner, phase);

    // Sync history overlay (on top of popup)
    if app.show_sync_history && is_finished {
        render_sync_history_overlay(app, frame, inner);
    }
}

// ── Popup layout ────────────────────────────────────────────────────────────

fn render_running_layout(app: &App, frame: &mut Frame, area: Rect, phase: f64) {
    let chunks = Layout::vertical([
        Constraint::Length(6), // Banner
        Constraint::Length(3), // Title
        Constraint::Length(3), // Progress bar
        Constraint::Length(1), // Enriched counters / summary
        Constraint::Length(1), // Throughput / performance
        Constraint::Length(1), // Phase / filter
        Constraint::Length(1), // Worker slots / status
        Constraint::Min(5),    // Log (running or completed)
        Constraint::Length(2), // Bottom actions + nav
    ])
    .split(area);

    render_animated_banner(frame, chunks[0], phase);
    render_title(app, frame, chunks[1]);
    render_progress_bar(app, frame, chunks[2]);
    render_enriched_counters(app, frame, chunks[3]);
    render_throughput(app, frame, chunks[4]);
    render_phase_indicator(app, frame, chunks[5]);
    render_worker_slots(app, frame, chunks[6]);
    render_main_log(app, frame, chunks[7]);
    render_bottom_actions(app, frame, chunks[8]);
}

fn render_main_log(app: &App, frame: &mut Frame, area: Rect) {
    if matches!(app.operation_state, OperationState::Finished { .. }) {
        render_filterable_log(app, frame, area);
    } else {
        render_running_log(app, frame, area);
    }
}

fn render_bottom_actions(app: &App, frame: &mut Frame, area: Rect) {
    let rows = Layout::vertical([Constraint::Length(1), Constraint::Length(1)]).split(area);

    let dim = Style::default().fg(Color::DarkGray);
    let key_style = Style::default()
        .fg(Color::Rgb(37, 99, 235))
        .add_modifier(Modifier::BOLD);

    let mut action_spans = vec![
        Span::styled("[s]", key_style),
        Span::styled(" Start Sync", dim),
        Span::raw("   "),
        Span::styled("[p]", key_style),
        Span::styled(" Hide Sync Progress", dim),
    ];

    if matches!(app.operation_state, OperationState::Finished { .. }) {
        action_spans.extend([
            Span::raw("   "),
            Span::styled("[a]", key_style),
            Span::styled(" All", dim),
            Span::raw(" "),
            Span::styled("[u]", key_style),
            Span::styled(" Updated", dim),
            Span::raw(" "),
            Span::styled("[f]", key_style),
            Span::styled(" Failed", dim),
            Span::raw(" "),
            Span::styled("[x]", key_style),
            Span::styled(" Skipped", dim),
            Span::raw(" "),
            Span::styled("[c]", key_style),
            Span::styled(" Changelog", dim),
            Span::raw(" "),
            Span::styled("[h]", key_style),
            Span::styled(" History", dim),
        ]);
    }
    frame.render_widget(
        Paragraph::new(vec![Line::from(action_spans)]).centered(),
        rows[0],
    );

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

    frame.render_widget(Paragraph::new(vec![Line::from(left_spans)]), nav_cols[0]);
    frame.render_widget(
        Paragraph::new(vec![Line::from(right_spans)]).right_aligned(),
        nav_cols[1],
    );
}

fn centered_rect(area: Rect, width_percent: u16, height_percent: u16) -> Rect {
    let width = (area.width.saturating_mul(width_percent) / 100).max(1);
    let height = (area.height.saturating_mul(height_percent) / 100).max(1);
    let x = area.x + (area.width.saturating_sub(width) / 2);
    let y = area.y + (area.height.saturating_sub(height) / 2);
    Rect::new(x, y, width, height)
}

fn dim_outside_popup(frame: &mut Frame, popup: Rect) {
    let area = frame.area();
    let popup_right = popup.x.saturating_add(popup.width);
    let popup_bottom = popup.y.saturating_add(popup.height);

    let buf = frame.buffer_mut();
    for y in area.y..area.y.saturating_add(area.height) {
        for x in area.x..area.x.saturating_add(area.width) {
            let inside_popup = x >= popup.x && x < popup_right && y >= popup.y && y < popup_bottom;
            if inside_popup {
                continue;
            }
            if let Some(cell) = buf.cell_mut(Position::new(x, y)) {
                cell.set_style(
                    Style::default()
                        .fg(Color::DarkGray)
                        .add_modifier(Modifier::DIM),
                );
            }
        }
    }
}

// ── Shared render functions ─────────────────────────────────────────────────

fn render_title(app: &App, frame: &mut Frame, area: Rect) {
    let title_text = match &app.operation_state {
        OperationState::Idle => "Sync Progress".to_string(),
        OperationState::Discovering { .. } | OperationState::Running { .. } => {
            "Sync Running".to_string()
        }
        OperationState::Finished { .. } => "Sync Completed".to_string(),
    };

    let style = match &app.operation_state {
        OperationState::Finished { .. } => Style::default().fg(Color::Rgb(21, 128, 61)),
        OperationState::Running { .. } => Style::default().fg(Color::Cyan),
        _ => Style::default().fg(Color::Yellow),
    };

    let title = Paragraph::new(Line::from(Span::styled(
        title_text,
        style.add_modifier(Modifier::BOLD),
    )))
    .centered()
    .block(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    frame.render_widget(title, area);
}

fn render_progress_bar(app: &App, frame: &mut Frame, area: Rect) {
    let (ratio, label) = match &app.operation_state {
        OperationState::Running {
            total, completed, ..
        } => {
            let r = if *total > 0 {
                *completed as f64 / *total as f64
            } else {
                0.0
            };
            let pct = (r * 100.0) as u32;
            (r, format!("{}/{} ({}%)", completed, total, pct))
        }
        OperationState::Finished { .. } => (1.0, "Done".to_string()),
        OperationState::Discovering { .. } => (0.0, "Discovering repositories...".to_string()),
        OperationState::Idle => (0.0, "Press [s] to start sync".to_string()),
    };

    let gauge = Gauge::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        )
        .gauge_style(Style::default().fg(Color::Cyan))
        .ratio(ratio.clamp(0.0, 1.0))
        .label(label);
    frame.render_widget(gauge, area);
}

// ── During-sync specific renders ────────────────────────────────────────────

fn render_enriched_counters(app: &App, frame: &mut Frame, area: Rect) {
    match &app.operation_state {
        OperationState::Running {
            completed,
            failed,
            skipped,
            with_updates,
            cloned,
            current_repo,
            ..
        } => {
            let up_to_date = completed
                .saturating_sub(*failed)
                .saturating_sub(*skipped)
                .saturating_sub(*with_updates)
                .saturating_sub(*cloned);

            let mut spans = vec![
                Span::raw("  "),
                Span::styled("Updated: ", Style::default().fg(Color::Yellow)),
                Span::styled(
                    with_updates.to_string(),
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw("  "),
                Span::styled("Current: ", Style::default().fg(Color::Rgb(21, 128, 61))),
                Span::styled(
                    up_to_date.to_string(),
                    Style::default().fg(Color::Rgb(21, 128, 61)),
                ),
                Span::raw("  "),
                Span::styled("Cloned: ", Style::default().fg(Color::Cyan)),
                Span::styled(cloned.to_string(), Style::default().fg(Color::Cyan)),
            ];

            if *failed > 0 {
                spans.push(Span::raw("  "));
                spans.push(Span::styled("Failed: ", Style::default().fg(Color::Red)));
                spans.push(Span::styled(
                    failed.to_string(),
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                ));
            }

            if *skipped > 0 {
                spans.push(Span::raw("  "));
                spans.push(Span::styled(
                    "Skipped: ",
                    Style::default().fg(Color::DarkGray),
                ));
                spans.push(Span::styled(
                    skipped.to_string(),
                    Style::default().fg(Color::DarkGray),
                ));
            }

            if !current_repo.is_empty() {
                spans.push(Span::raw("  "));
                spans.push(Span::styled(
                    current_repo.as_str(),
                    Style::default().fg(Color::DarkGray),
                ));
            }

            frame.render_widget(Paragraph::new(Line::from(spans)), area);
        }
        OperationState::Finished {
            summary,
            with_updates,
            cloned,
            ..
        } => {
            let current = summary
                .success
                .saturating_sub(*with_updates)
                .saturating_sub(*cloned);

            let spans = vec![
                Span::raw("  "),
                Span::styled("Updated: ", Style::default().fg(Color::Yellow)),
                Span::styled(
                    with_updates.to_string(),
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw("  "),
                Span::styled("Current: ", Style::default().fg(Color::Rgb(21, 128, 61))),
                Span::styled(
                    current.to_string(),
                    Style::default().fg(Color::Rgb(21, 128, 61)),
                ),
                Span::raw("  "),
                Span::styled("Cloned: ", Style::default().fg(Color::Cyan)),
                Span::styled(cloned.to_string(), Style::default().fg(Color::Cyan)),
                Span::raw("  "),
                Span::styled("Failed: ", Style::default().fg(Color::Red)),
                Span::styled(
                    summary.failed.to_string(),
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                ),
                Span::raw("  "),
                Span::styled("Skipped: ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    summary.skipped.to_string(),
                    Style::default().fg(Color::DarkGray),
                ),
            ];
            frame.render_widget(Paragraph::new(Line::from(spans)), area);
        }
        OperationState::Discovering { message, .. } => {
            frame.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::raw("  "),
                    Span::styled("Discovering: ", Style::default().fg(Color::Yellow)),
                    Span::styled(message.as_str(), Style::default().fg(Color::DarkGray)),
                ])),
                area,
            );
        }
        OperationState::Idle => {
            frame.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(
                        "No sync activity yet.",
                        Style::default().fg(Color::DarkGray),
                    ),
                ])),
                area,
            );
        }
    }
}

fn render_throughput(app: &App, frame: &mut Frame, area: Rect) {
    match &app.operation_state {
        OperationState::Running {
            completed,
            total,
            started_at,
            throughput_samples,
            ..
        } => {
            let elapsed = started_at.elapsed();
            let elapsed_secs = elapsed.as_secs_f64();
            let repos_per_sec = if elapsed_secs > 1.0 {
                *completed as f64 / elapsed_secs
            } else {
                0.0
            };
            let remaining = total.saturating_sub(*completed);
            let eta_secs = if repos_per_sec > 0.1 {
                (remaining as f64 / repos_per_sec).ceil() as u64
            } else {
                0
            };

            let mut spans = vec![
                Span::raw("  "),
                Span::styled("Elapsed: ", Style::default().fg(Color::DarkGray)),
                Span::styled(format_duration(elapsed), Style::default().fg(Color::Cyan)),
            ];

            if repos_per_sec > 0.0 {
                spans.push(Span::raw("  "));
                spans.push(Span::styled(
                    format!("~{:.1} repos/sec", repos_per_sec),
                    Style::default().fg(Color::DarkGray),
                ));
            }

            let has_eta_data = throughput_samples.iter().any(|&sample| sample > 0);
            if has_eta_data && eta_secs > 0 && *completed > 0 {
                spans.push(Span::raw("  "));
                spans.push(Span::styled("ETA: ", Style::default().fg(Color::DarkGray)));
                spans.push(Span::styled(
                    format!("~{}s", eta_secs),
                    Style::default().fg(Color::Cyan),
                ));
            }

            // Add sparkline inline if we have samples.
            if !throughput_samples.is_empty() {
                spans.push(Span::raw("  "));
                let max_val = throughput_samples.iter().copied().max().unwrap_or(1).max(1);
                let bars = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
                let spark_str: String = throughput_samples
                    .iter()
                    .rev()
                    .take(20)
                    .collect::<Vec<_>>()
                    .iter()
                    .rev()
                    .map(|&v| {
                        let idx = ((*v as f64 / max_val as f64) * 7.0) as usize;
                        bars[idx.min(7)]
                    })
                    .collect();
                spans.push(Span::styled(spark_str, Style::default().fg(Color::Cyan)));
            }

            frame.render_widget(Paragraph::new(Line::from(spans)), area);
        }
        OperationState::Finished { .. } => {
            render_performance_line(app, frame, area);
        }
        OperationState::Discovering { .. } => {
            frame.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(
                        "Building sync plan...",
                        Style::default().fg(Color::DarkGray),
                    ),
                ])),
                area,
            );
        }
        OperationState::Idle => {
            frame.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(
                        "Press [p] to hide, [s] to start.",
                        Style::default().fg(Color::DarkGray),
                    ),
                ])),
                area,
            );
        }
    }
}

fn render_phase_indicator(app: &App, frame: &mut Frame, area: Rect) {
    match &app.operation_state {
        OperationState::Running {
            to_clone,
            to_sync,
            cloned,
            synced,
            ..
        } => {
            if *to_clone == 0 && *to_sync == 0 {
                return;
            }

            let mut spans = vec![Span::raw("  Phase: ")];

            if *to_clone > 0 {
                let clone_pct = if *to_clone > 0 {
                    *cloned as f64 / *to_clone as f64
                } else {
                    0.0
                };
                let bar_width: usize = 8;
                let filled = (clone_pct * bar_width as f64).round() as usize;
                spans.push(Span::styled(
                    "\u{2588}".repeat(filled),
                    Style::default().fg(Color::Cyan),
                ));
                spans.push(Span::styled(
                    "\u{2591}".repeat(bar_width.saturating_sub(filled)),
                    Style::default().fg(Color::DarkGray),
                ));
                spans.push(Span::styled(
                    format!(" Clone {}/{}", cloned, to_clone),
                    Style::default().fg(Color::Cyan),
                ));
                spans.push(Span::raw("  "));
            }

            if *to_sync > 0 {
                let sync_pct = if *to_sync > 0 {
                    *synced as f64 / *to_sync as f64
                } else {
                    0.0
                };
                let bar_width: usize = 12;
                let filled = (sync_pct * bar_width as f64).round() as usize;
                spans.push(Span::styled(
                    "\u{2588}".repeat(filled),
                    Style::default().fg(Color::Rgb(21, 128, 61)),
                ));
                spans.push(Span::styled(
                    "\u{2591}".repeat(bar_width.saturating_sub(filled)),
                    Style::default().fg(Color::DarkGray),
                ));
                spans.push(Span::styled(
                    format!(" Sync {}/{}", synced, to_sync),
                    Style::default().fg(Color::Rgb(21, 128, 61)),
                ));
            }

            frame.render_widget(Paragraph::new(Line::from(spans)), area);
        }
        OperationState::Finished { .. } => {
            let label = match app.log_filter {
                LogFilter::All => "All",
                LogFilter::Updated => "Updated",
                LogFilter::Failed => "Failed",
                LogFilter::Skipped => "Skipped",
                LogFilter::Changelog => "Changelog",
            };

            let spans = vec![
                Span::raw("  "),
                Span::styled("Filter: ", Style::default().fg(Color::DarkGray)),
                Span::styled(label, Style::default().fg(Color::Cyan)),
                Span::styled("  |  ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    format!("{} entries", filtered_log_count(app)),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled("  |  ", Style::default().fg(Color::DarkGray)),
                Span::styled("[←]/[→]", Style::default().fg(Color::Rgb(37, 99, 235))),
                Span::styled(" filter", Style::default().fg(Color::DarkGray)),
            ];
            frame.render_widget(Paragraph::new(Line::from(spans)), area);
        }
        _ => {}
    }
}

fn render_worker_slots(app: &App, frame: &mut Frame, area: Rect) {
    match &app.operation_state {
        OperationState::Running { active_repos, .. } => {
            if active_repos.is_empty() {
                frame.render_widget(
                    Paragraph::new(Line::from(vec![
                        Span::raw("  "),
                        Span::styled("Workers idle", Style::default().fg(Color::DarkGray)),
                    ])),
                    area,
                );
                return;
            }

            let mut spans = vec![Span::raw("  ")];
            for (i, repo) in active_repos.iter().enumerate() {
                if i > 0 {
                    spans.push(Span::raw("  "));
                }
                spans.push(Span::styled(
                    format!("[{}]", i + 1),
                    Style::default()
                        .fg(Color::DarkGray)
                        .add_modifier(Modifier::BOLD),
                ));
                spans.push(Span::raw(" "));
                // Show just the repo name (not org/) to save space.
                let short = repo.split('/').next_back().unwrap_or(repo);
                spans.push(Span::styled(short, Style::default().fg(Color::Cyan)));
            }

            frame.render_widget(Paragraph::new(Line::from(spans)), area);
        }
        OperationState::Finished {
            total_new_commits, ..
        } => {
            let mut spans = vec![
                Span::raw("  "),
                Span::styled(
                    "Completed. ",
                    Style::default()
                        .fg(Color::Rgb(21, 128, 61))
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled("[↑]/[↓] move", Style::default().fg(Color::Rgb(37, 99, 235))),
                Span::styled("  ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    "[Enter] commit details",
                    Style::default().fg(Color::Rgb(37, 99, 235)),
                ),
            ];

            if *total_new_commits > 0 {
                spans.push(Span::styled(
                    format!("  |  {} new commits", total_new_commits),
                    Style::default().fg(Color::Yellow),
                ));
            }

            frame.render_widget(Paragraph::new(Line::from(spans)), area);
        }
        OperationState::Discovering { .. } => {
            frame.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(
                        "Waiting for workers...",
                        Style::default().fg(Color::DarkGray),
                    ),
                ])),
                area,
            );
        }
        OperationState::Idle => {
            frame.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(
                        "Use [p] to close this popup.",
                        Style::default().fg(Color::DarkGray),
                    ),
                ])),
                area,
            );
        }
    }
}

fn render_running_log(app: &App, frame: &mut Frame, area: Rect) {
    if app.log_lines.is_empty() {
        let message = match app.operation_state {
            OperationState::Idle => "  No sync activity yet. Press [s] to start sync.",
            OperationState::Discovering { .. } => "  Discovering repositories...",
            _ => "  Waiting for log output...",
        };
        let empty = Paragraph::new(Line::from(Span::styled(
            message,
            Style::default().fg(Color::DarkGray),
        )))
        .block(
            Block::default()
                .title(" Log ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        );
        frame.render_widget(empty, area);
        return;
    }

    let visible_height = area.height.saturating_sub(2) as usize;
    let total = app.log_lines.len();
    let max_start = total.saturating_sub(visible_height);
    let start = app.scroll_offset.min(max_start);
    let end = (start + visible_height).min(total);

    let items: Vec<ListItem> = app.log_lines[start..end]
        .iter()
        .map(|line| {
            let style = if line.starts_with("[**]") {
                Style::default().fg(Color::Yellow)
            } else if line.starts_with("[++]") {
                Style::default().fg(Color::Cyan)
            } else if line.starts_with("[ok]") {
                Style::default().fg(Color::Rgb(21, 128, 61))
            } else if line.starts_with("[!!]") {
                Style::default().fg(Color::Red)
            } else if line.starts_with("[--]") {
                Style::default().fg(Color::DarkGray)
            } else {
                Style::default()
            };
            ListItem::new(Line::from(Span::styled(format!("  {}", line), style)))
        })
        .collect();

    let log = List::new(items).block(
        Block::default()
            .title(" Log ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    frame.render_widget(log, area);
}

// ── Post-sync specific renders ──────────────────────────────────────────────

fn render_performance_line(app: &App, frame: &mut Frame, area: Rect) {
    if let OperationState::Finished {
        summary,
        duration_secs,
        total_new_commits,
        cloned,
        ..
    } = &app.operation_state
    {
        let total = summary.success + summary.failed + summary.skipped;
        let repos_per_sec = if *duration_secs > 0.0 {
            total as f64 / duration_secs
        } else {
            0.0
        };

        let mut spans = vec![
            Span::raw("  "),
            Span::styled(
                format!("{} repos", total),
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(" in ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{:.1}s", duration_secs),
                Style::default().fg(Color::Cyan),
            ),
            Span::styled(
                format!(" ({:.1} repos/sec)", repos_per_sec),
                Style::default().fg(Color::DarkGray),
            ),
        ];

        if *total_new_commits > 0 {
            spans.push(Span::styled(
                format!(" \u{00b7} {} new commits", total_new_commits),
                Style::default().fg(Color::Yellow),
            ));
        }

        if *cloned > 0 {
            spans.push(Span::styled(
                format!(" \u{00b7} {} cloned", cloned),
                Style::default().fg(Color::Cyan),
            ));
        }

        frame.render_widget(Paragraph::new(Line::from(spans)), area);
    }
}

fn render_filterable_log(app: &App, frame: &mut Frame, area: Rect) {
    // Changelog mode has its own renderer
    if app.log_filter == LogFilter::Changelog {
        render_changelog(app, frame, area);
        return;
    }

    let entries: Vec<&crate::tui::app::SyncLogEntry> = match app.log_filter {
        LogFilter::All => app.sync_log_entries.iter().collect(),
        LogFilter::Updated => app
            .sync_log_entries
            .iter()
            .filter(|e| e.had_updates || e.is_clone)
            .collect(),
        LogFilter::Failed => app
            .sync_log_entries
            .iter()
            .filter(|e| e.status == SyncLogStatus::Failed)
            .collect(),
        LogFilter::Skipped => app
            .sync_log_entries
            .iter()
            .filter(|e| e.status == SyncLogStatus::Skipped)
            .collect(),
        LogFilter::Changelog => app
            .sync_log_entries
            .iter()
            .filter(|e| e.had_updates)
            .collect(),
    };

    let visible_height = area.height.saturating_sub(2) as usize;
    let total_entries = entries.len();

    // Ensure scroll index is in bounds
    let scroll_start = if total_entries > visible_height {
        let max_start = total_entries.saturating_sub(visible_height);
        app.sync_log_index.min(max_start)
    } else {
        0
    };

    let mut items: Vec<ListItem> = Vec::new();
    let is_expanded = app.expanded_repo.is_some();

    for (i, entry) in entries
        .iter()
        .skip(scroll_start)
        .take(visible_height)
        .enumerate()
    {
        let (prefix, color) = match entry.status {
            SyncLogStatus::Updated => ("[**]", Color::Yellow),
            SyncLogStatus::Cloned => ("[++]", Color::Cyan),
            SyncLogStatus::Success => ("[ok]", Color::Rgb(21, 128, 61)),
            SyncLogStatus::Failed => ("[!!]", Color::Red),
            SyncLogStatus::Skipped => ("[--]", Color::DarkGray),
        };

        let is_selected = i + scroll_start == app.sync_log_index;
        let this_expanded = is_expanded && app.expanded_repo.as_deref() == Some(&entry.repo_name);
        let style = if is_selected {
            Style::default().fg(color).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(color)
        };

        let indicator = if this_expanded {
            " v "
        } else if is_selected {
            " > "
        } else {
            "   "
        };

        let mut spans = vec![
            Span::styled(indicator, style),
            Span::styled(prefix, style),
            Span::raw(" "),
            Span::styled(&entry.repo_name, style),
        ];

        // Add detail based on status
        match entry.status {
            SyncLogStatus::Updated | SyncLogStatus::Cloned => {
                spans.push(Span::styled(
                    format!(" - {}", entry.message),
                    Style::default().fg(Color::DarkGray),
                ));
                if let Some(n) = entry.new_commits {
                    if n > 0 {
                        spans.push(Span::styled(
                            format!(" ({} new commits)", n),
                            Style::default().fg(Color::DarkGray),
                        ));
                    }
                }
            }
            _ => {
                spans.push(Span::styled(
                    format!(" - {}", entry.message),
                    Style::default().fg(Color::DarkGray),
                ));
            }
        }

        items.push(ListItem::new(Line::from(spans)));

        // Render expanded commits inline below this entry
        if this_expanded {
            if app.repo_commits.is_empty() {
                items.push(ListItem::new(Line::from(vec![
                    Span::raw("      "),
                    Span::styled(
                        "Loading...",
                        Style::default()
                            .fg(Color::DarkGray)
                            .add_modifier(Modifier::ITALIC),
                    ),
                ])));
            } else {
                let max_commits = visible_height.saturating_sub(items.len()).max(3);
                for commit in app.repo_commits.iter().take(max_commits) {
                    items.push(ListItem::new(Line::from(vec![
                        Span::raw("      "),
                        Span::styled(commit, Style::default().fg(Color::DarkGray)),
                    ])));
                }
                if app.repo_commits.len() > max_commits {
                    items.push(ListItem::new(Line::from(vec![
                        Span::raw("      "),
                        Span::styled(
                            format!("... and {} more", app.repo_commits.len() - max_commits),
                            Style::default().fg(Color::DarkGray),
                        ),
                    ])));
                }
            }
        }
    }

    let filter_label = match app.log_filter {
        LogFilter::All => "All",
        LogFilter::Updated => "Updated",
        LogFilter::Failed => "Failed",
        LogFilter::Skipped => "Skipped",
        LogFilter::Changelog => "Changelog",
    };

    let title = format!(" Log [{}] ({}) ", filter_label, total_entries);

    let log = List::new(items).block(
        Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    frame.render_widget(log, area);
}

// ── Aggregate changelog ─────────────────────────────────────────────────────

const REPO_COLORS: [Color; 4] = [Color::Yellow, Color::Cyan, Color::Green, Color::Magenta];

fn render_changelog(app: &App, frame: &mut Frame, area: Rect) {
    let updated_repos: Vec<&crate::tui::app::SyncLogEntry> = app
        .sync_log_entries
        .iter()
        .filter(|e| e.had_updates)
        .collect();

    // Loading state
    if app.changelog_loaded < app.changelog_total && app.changelog_total > 0 {
        let loading = format!(
            "Fetching commits from {} updated repositories... {}/{}",
            app.changelog_total, app.changelog_loaded, app.changelog_total
        );
        let block = Block::default()
            .title(" Log [Changelog] ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray));
        let paragraph = Paragraph::new(loading)
            .alignment(Alignment::Center)
            .block(block);
        frame.render_widget(paragraph, area);
        return;
    }

    // Empty state
    if updated_repos.is_empty() {
        let block = Block::default()
            .title(" Log [Changelog] ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray));
        let paragraph = Paragraph::new("No updated repositories")
            .alignment(Alignment::Center)
            .block(block);
        frame.render_widget(paragraph, area);
        return;
    }

    // Build grouped timeline items
    let mut items: Vec<ListItem> = Vec::new();
    let total_commits: usize = app.changelog_commits.values().map(|v| v.len()).sum();

    for (i, entry) in updated_repos.iter().enumerate() {
        let color = REPO_COLORS[i % REPO_COLORS.len()];
        let commits = app.changelog_commits.get(&entry.repo_name);
        let count = commits.map(|c| c.len()).unwrap_or(0);

        // Repo header: ● repo/name ··················· N commits
        let header_right = format!("{} commits ", count);
        let used: u16 = 6 + entry.repo_name.len() as u16 + header_right.len() as u16;
        let padding = area.width.saturating_sub(used + 2) as usize;
        let dots = "·".repeat(padding);

        items.push(ListItem::new(Line::from(vec![
            Span::styled(
                "  ● ",
                Style::default().fg(color).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                entry.repo_name.as_str(),
                Style::default().fg(color).add_modifier(Modifier::BOLD),
            ),
            Span::styled(format!(" {} ", dots), Style::default().fg(Color::DarkGray)),
            Span::styled(header_right, Style::default().fg(Color::DarkGray)),
        ])));

        // Commit lines with │ connector
        if let Some(commits) = commits {
            for (j, commit) in commits.iter().enumerate() {
                let connector = if j < commits.len() - 1 { "│" } else { " " };
                items.push(ListItem::new(Line::from(vec![
                    Span::styled(format!("  {connector}  "), Style::default().fg(color)),
                    Span::styled(commit.as_str(), Style::default().fg(Color::DarkGray)),
                ])));
            }
        }

        // Blank separator between repos (except last)
        if i < updated_repos.len() - 1 {
            items.push(ListItem::new(Line::from("")));
        }
    }

    let visible_height = area.height.saturating_sub(2) as usize;
    let total_lines = items.len();
    let max_scroll = total_lines.saturating_sub(visible_height);
    let scroll = app.changelog_scroll.min(max_scroll);

    let title = format!(
        " Log [Changelog] ({} commits across {} repos) ",
        total_commits,
        updated_repos.len()
    );

    let items: Vec<ListItem> = items
        .into_iter()
        .skip(scroll)
        .take(visible_height)
        .collect();

    let list = List::new(items).block(
        Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    frame.render_widget(list, area);
}

// ── Sync history overlay ────────────────────────────────────────────────────

fn render_sync_history_overlay(app: &App, frame: &mut Frame, area: Rect) {
    if app.sync_history.is_empty() {
        return;
    }

    let overlay_height = (app.sync_history.len() as u16 + 2).min(14);
    let overlay_width = 60u16.min(area.width.saturating_sub(4));

    let x = area.x + area.width.saturating_sub(overlay_width) / 2;
    let y = area.y + area.height.saturating_sub(overlay_height) / 2;
    let overlay_area = Rect::new(x, y, overlay_width, overlay_height);

    frame.render_widget(Clear, overlay_area);

    let items: Vec<ListItem> = app
        .sync_history
        .iter()
        .rev()
        .map(|entry| {
            // Parse and format timestamp
            let time_str = if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&entry.timestamp) {
                dt.format("%b %d, %H:%M").to_string()
            } else {
                "unknown".to_string()
            };

            let total = entry.success + entry.failed + entry.skipped;
            let mut spans = vec![
                Span::raw("  "),
                Span::styled(
                    format!("{:<14}", time_str),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(
                    format!("{:>3} repos", total),
                    Style::default().fg(Color::Cyan),
                ),
                Span::raw("  "),
            ];

            if entry.with_updates > 0 {
                spans.push(Span::styled(
                    format!("{} updated", entry.with_updates),
                    Style::default().fg(Color::Yellow),
                ));
            } else if entry.cloned > 0 {
                spans.push(Span::styled(
                    format!("{} cloned", entry.cloned),
                    Style::default().fg(Color::Cyan),
                ));
            } else {
                spans.push(Span::styled(
                    "no changes",
                    Style::default().fg(Color::DarkGray),
                ));
            }

            spans.push(Span::raw("  "));
            spans.push(Span::styled(
                format!("{:.1}s", entry.duration_secs),
                Style::default().fg(Color::DarkGray),
            ));

            ListItem::new(Line::from(spans))
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .title(" Sync History ")
            .borders(Borders::ALL)
            .border_type(BorderType::Thick)
            .border_style(Style::default().fg(Color::Cyan)),
    );
    frame.render_widget(list, overlay_area);
}

// ── Utilities ───────────────────────────────────────────────────────────────

fn format_duration(d: std::time::Duration) -> String {
    let secs = d.as_secs();
    if secs >= 60 {
        format!("{}m{}s", secs / 60, secs % 60)
    } else {
        format!("{}s", secs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, WorkspaceConfig};
    use crate::tui::app::{Operation, Screen};
    use crate::types::OpSummary;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use tokio::sync::mpsc::unbounded_channel;

    fn build_app() -> App {
        let ws = WorkspaceConfig::new("test-ws", "/tmp/test-ws");
        let mut app = App::new(Config::default(), vec![ws]);
        app.screen = Screen::Sync;
        app.screen_stack = vec![Screen::Dashboard];
        app
    }

    #[test]
    fn sync_key_p_hides_progress_popup() {
        let mut app = build_app();
        let (tx, _rx) = unbounded_channel();
        app.scroll_offset = 5;

        handle_key(
            &mut app,
            KeyEvent::new(KeyCode::Char('p'), KeyModifiers::NONE),
            &tx,
        );

        assert_eq!(app.screen, Screen::Dashboard);
        assert_eq!(app.scroll_offset, 5);
    }

    #[tokio::test]
    async fn sync_key_s_starts_sync() {
        let mut app = build_app();
        let (tx, _rx) = unbounded_channel();

        handle_key(
            &mut app,
            KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE),
            &tx,
        );

        assert_eq!(app.screen, Screen::Sync);
        assert!(matches!(
            app.operation_state,
            OperationState::Discovering {
                operation: Operation::Sync,
                ..
            }
        ));
    }

    #[test]
    fn right_arrow_cycles_finished_filter() {
        let mut app = build_app();
        let (tx, _rx) = unbounded_channel();
        app.operation_state = OperationState::Finished {
            operation: Operation::Sync,
            summary: OpSummary {
                success: 1,
                failed: 0,
                skipped: 0,
            },
            with_updates: 0,
            cloned: 0,
            synced: 1,
            total_new_commits: 0,
            duration_secs: 1.0,
        };
        app.log_filter = LogFilter::All;

        handle_key(
            &mut app,
            KeyEvent::new(KeyCode::Right, KeyModifiers::NONE),
            &tx,
        );

        assert_eq!(app.log_filter, LogFilter::Updated);
    }
}
