//! Sync progress screen — real-time metrics during sync, enriched summary after.

use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Gauge, List, ListItem, Paragraph},
    Frame,
};

use crate::tui::app::{App, LogFilter, OperationState, SyncLogStatus};
use crate::tui::widgets::status_bar;

use crate::banner::render_animated_banner;

pub fn render(app: &App, frame: &mut Frame) {
    let is_finished = matches!(&app.operation_state, OperationState::Finished { .. });

    // Animate during active ops, static otherwise
    let phase = match &app.operation_state {
        OperationState::Discovering { .. } | OperationState::Running { .. } => {
            (app.tick_count as f64 / 50.0).fract()
        }
        _ => 0.0,
    };

    if is_finished {
        render_finished_layout(app, frame, phase);
    } else {
        render_running_layout(app, frame, phase);
    }

    // Sync history overlay (on top of everything)
    if app.show_sync_history && is_finished {
        render_sync_history_overlay(app, frame);
    }
}

// ── During-sync layout ──────────────────────────────────────────────────────

fn render_running_layout(app: &App, frame: &mut Frame, phase: f64) {
    let chunks = Layout::vertical([
        Constraint::Length(6), // Banner
        Constraint::Length(3), // Title
        Constraint::Length(3), // Progress bar
        Constraint::Length(1), // Enriched counters
        Constraint::Length(1), // Throughput/ETA
        Constraint::Length(1), // Phase indicator
        Constraint::Length(1), // Worker slots
        Constraint::Min(5),    // Log
        Constraint::Length(1), // Status bar
    ])
    .split(frame.area());

    render_animated_banner(frame, chunks[0], phase);
    render_title(app, frame, chunks[1]);
    render_progress_bar(app, frame, chunks[2]);
    render_enriched_counters(app, frame, chunks[3]);
    render_throughput(app, frame, chunks[4]);
    render_phase_indicator(app, frame, chunks[5]);
    render_worker_slots(app, frame, chunks[6]);
    render_running_log(app, frame, chunks[7]);

    let hint = match &app.operation_state {
        OperationState::Running { .. } => "j/k: Scroll log  Ctrl+C: Quit",
        _ => "Ctrl+C: Quit",
    };
    status_bar::render(frame, chunks[8], hint);
}

// ── Post-sync layout ────────────────────────────────────────────────────────

fn render_finished_layout(app: &App, frame: &mut Frame, phase: f64) {
    // Check if "nothing changed"
    let is_empty = matches!(
        &app.operation_state,
        OperationState::Finished {
            with_updates: 0,
            cloned: 0,
            ..
        } if app.sync_log_entries.iter().all(|e| e.status != SyncLogStatus::Failed)
    );

    if is_empty {
        render_nothing_changed_layout(app, frame, phase);
        return;
    }

    let chunks = Layout::vertical([
        Constraint::Length(6), // Banner
        Constraint::Length(3), // Title
        Constraint::Length(3), // Progress bar (done)
        Constraint::Length(4), // Stat boxes
        Constraint::Length(1), // Performance line
        Constraint::Min(5),    // Filterable log
        Constraint::Length(1), // Status bar
    ])
    .split(frame.area());

    render_animated_banner(frame, chunks[0], phase);
    render_title(app, frame, chunks[1]);
    render_progress_bar(app, frame, chunks[2]);
    render_summary_boxes(app, frame, chunks[3]);
    render_performance_line(app, frame, chunks[4]);
    render_filterable_log(app, frame, chunks[5]);
    status_bar::render(
        frame,
        chunks[6],
        "Esc: Back  qq: Quit  Enter: Commits  a:All u:Upd f:Err x:Skip h:History",
    );
}

// ── "Nothing changed" layout ────────────────────────────────────────────────

fn render_nothing_changed_layout(app: &App, frame: &mut Frame, phase: f64) {
    let chunks = Layout::vertical([
        Constraint::Length(6), // Banner
        Constraint::Length(3), // Title
        Constraint::Length(3), // Progress bar (done)
        Constraint::Min(5),    // Empty state message
        Constraint::Length(1), // Performance line
        Constraint::Length(1), // Status bar
    ])
    .split(frame.area());

    render_animated_banner(frame, chunks[0], phase);
    render_title(app, frame, chunks[1]);
    render_progress_bar(app, frame, chunks[2]);

    // Friendly empty state
    if let OperationState::Finished { summary, .. } = &app.operation_state {
        let total = summary.success + summary.failed + summary.skipped;
        let msg = Paragraph::new(vec![
            Line::from(""),
            Line::from(""),
            Line::from(Span::styled(
                "Everything up to date",
                Style::default()
                    .fg(Color::Rgb(21, 128, 61))
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                format!("{} repositories synced, no changes found", total),
                Style::default().fg(Color::DarkGray),
            )),
        ])
        .centered();
        frame.render_widget(msg, chunks[3]);
    }

    render_performance_line(app, frame, chunks[4]);
    status_bar::render(frame, chunks[5], "Esc: Back  qq: Quit  h: History");
}

// ── Shared render functions ─────────────────────────────────────────────────

fn render_title(app: &App, frame: &mut Frame, area: Rect) {
    let title_text = match &app.operation_state {
        OperationState::Idle => "Idle".to_string(),
        OperationState::Discovering { message } => message.clone(),
        OperationState::Running { operation, .. } => format!("{}ing Repositories", operation),
        OperationState::Finished { operation, .. } => format!("{} Complete", operation),
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
        OperationState::Discovering { .. } => (0.0, "Discovering...".to_string()),
        OperationState::Idle => (0.0, String::new()),
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
    let (updated, up_to_date, cloned, failed, skipped, current) = match &app.operation_state {
        OperationState::Running {
            completed,
            failed,
            skipped,
            with_updates,
            cloned,
            current_repo,
            ..
        } => {
            let up = completed
                .saturating_sub(*failed)
                .saturating_sub(*skipped)
                .saturating_sub(*with_updates)
                .saturating_sub(*cloned);
            (
                *with_updates,
                up,
                *cloned,
                *failed,
                *skipped,
                current_repo.as_str(),
            )
        }
        _ => (0, 0, 0, 0, 0, ""),
    };

    let mut spans = vec![
        Span::raw("  "),
        Span::styled("Updated: ", Style::default().fg(Color::Yellow)),
        Span::styled(
            updated.to_string(),
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

    if failed > 0 {
        spans.push(Span::raw("  "));
        spans.push(Span::styled("Failed: ", Style::default().fg(Color::Red)));
        spans.push(Span::styled(
            failed.to_string(),
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        ));
    }

    if skipped > 0 {
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

    if !current.is_empty() {
        spans.push(Span::raw("  "));
        spans.push(Span::styled(current, Style::default().fg(Color::DarkGray)));
    }

    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn render_throughput(app: &App, frame: &mut Frame, area: Rect) {
    if let OperationState::Running {
        completed,
        total,
        started_at,
        throughput_samples,
        ..
    } = &app.operation_state
    {
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

        if eta_secs > 0 && *completed > 0 {
            spans.push(Span::raw("  "));
            spans.push(Span::styled("ETA: ", Style::default().fg(Color::DarkGray)));
            spans.push(Span::styled(
                format!("~{}s", eta_secs),
                Style::default().fg(Color::Cyan),
            ));
        }

        // Add sparkline inline if we have samples
        if !throughput_samples.is_empty() {
            spans.push(Span::raw("  "));
            // Render sparkline as unicode bars inline
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
}

fn render_phase_indicator(app: &App, frame: &mut Frame, area: Rect) {
    if let OperationState::Running {
        to_clone,
        to_sync,
        cloned,
        synced,
        ..
    } = &app.operation_state
    {
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
}

fn render_worker_slots(app: &App, frame: &mut Frame, area: Rect) {
    if let OperationState::Running { active_repos, .. } = &app.operation_state {
        if active_repos.is_empty() {
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
            // Show just the repo name (not org/) to save space
            let short = repo.split('/').next_back().unwrap_or(repo);
            spans.push(Span::styled(short, Style::default().fg(Color::Cyan)));
        }

        frame.render_widget(Paragraph::new(Line::from(spans)), area);
    }
}

fn render_running_log(app: &App, frame: &mut Frame, area: Rect) {
    let visible_height = area.height.saturating_sub(2) as usize;
    let total = app.log_lines.len();
    let start = total.saturating_sub(visible_height);

    let items: Vec<ListItem> = app.log_lines[start..]
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

fn render_summary_boxes(app: &App, frame: &mut Frame, area: Rect) {
    if let OperationState::Finished {
        summary,
        with_updates,
        cloned,
        ..
    } = &app.operation_state
    {
        let has_failures = summary.failed > 0;
        let current_count = summary
            .success
            .saturating_sub(*with_updates)
            .saturating_sub(*cloned);

        let cols = Layout::horizontal([
            Constraint::Ratio(1, 4),
            Constraint::Ratio(1, 4),
            Constraint::Ratio(1, 4),
            Constraint::Ratio(1, 4),
        ])
        .split(area);

        render_summary_box(
            frame,
            cols[0],
            &with_updates.to_string(),
            "Updated",
            Color::Yellow,
        );

        if has_failures {
            render_summary_box(
                frame,
                cols[1],
                &summary.failed.to_string(),
                "Failed",
                Color::Red,
            );
        } else {
            render_summary_box(
                frame,
                cols[1],
                &current_count.to_string(),
                "Current",
                Color::Rgb(21, 128, 61),
            );
        }

        render_summary_box(frame, cols[2], &cloned.to_string(), "Cloned", Color::Cyan);

        render_summary_box(
            frame,
            cols[3],
            &summary.skipped.to_string(),
            "Skipped",
            Color::DarkGray,
        );
    }
}

fn render_summary_box(frame: &mut Frame, area: Rect, value: &str, label: &str, color: Color) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .border_style(Style::default().fg(color));
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

// ── Sync history overlay ────────────────────────────────────────────────────

fn render_sync_history_overlay(app: &App, frame: &mut Frame) {
    if app.sync_history.is_empty() {
        return;
    }

    let area = frame.area();
    let overlay_height = (app.sync_history.len() as u16 + 2).min(14);
    let overlay_width = 60u16.min(area.width.saturating_sub(4));

    let x = area.width.saturating_sub(overlay_width) / 2;
    let y = area.height.saturating_sub(overlay_height) / 2;
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
