//! Progress screen — shows operation progress with gauge and log.

use ratatui::{
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph},
    Frame,
};

use crate::tui::app::{App, OperationState};
use crate::tui::widgets::status_bar;

use super::dashboard::render_animated_banner;

pub fn render(app: &App, frame: &mut Frame) {
    let chunks = Layout::vertical([
        Constraint::Length(6), // Animated banner
        Constraint::Length(3), // Title
        Constraint::Length(3), // Progress bar
        Constraint::Length(3), // Counters
        Constraint::Min(5),    // Log
        Constraint::Length(1), // Status bar
    ])
    .split(frame.area());

    // Animate during active ops, static otherwise
    // One full cycle every ~5 seconds (50 ticks at 100ms tick rate)
    let phase = match &app.operation_state {
        OperationState::Discovering { .. } | OperationState::Running { .. } => {
            (app.tick_count as f64 / 50.0).fract()
        }
        _ => 0.0,
    };

    render_animated_banner(frame, chunks[0], phase);
    render_title(app, frame, chunks[1]);
    render_progress_bar(app, frame, chunks[2]);
    render_counters(app, frame, chunks[3]);
    render_log(app, frame, chunks[4]);

    let hint = match &app.operation_state {
        OperationState::Finished { .. } => "Esc: Back  qq: Quit",
        OperationState::Running { .. } => "j/k: Scroll log  Ctrl+C: Quit",
        _ => "Ctrl+C: Quit",
    };
    status_bar::render(frame, chunks[5], hint);
}

fn render_title(app: &App, frame: &mut Frame, area: ratatui::layout::Rect) {
    let title_text = match &app.operation_state {
        OperationState::Idle => "Idle".to_string(),
        OperationState::Discovering { message } => message.clone(),
        OperationState::Running { operation, .. } => format!("{}ing Repositories", operation),
        OperationState::Finished { operation, .. } => format!("{} Complete", operation),
    };

    let style = match &app.operation_state {
        OperationState::Finished { .. } => Style::default().fg(Color::Green),
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

fn render_progress_bar(app: &App, frame: &mut Frame, area: ratatui::layout::Rect) {
    let (ratio, label) = match &app.operation_state {
        OperationState::Running {
            total, completed, ..
        } => {
            let r = if *total > 0 {
                *completed as f64 / *total as f64
            } else {
                0.0
            };
            (r, format!("{}/{}", completed, total))
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

fn render_counters(app: &App, frame: &mut Frame, area: ratatui::layout::Rect) {
    let (success, failed, skipped, current) = match &app.operation_state {
        OperationState::Running {
            completed,
            failed,
            skipped,
            current_repo,
            ..
        } => (
            completed.saturating_sub(*failed).saturating_sub(*skipped),
            *failed,
            *skipped,
            current_repo.as_str(),
        ),
        OperationState::Finished { summary, .. } => {
            (summary.success, summary.failed, summary.skipped, "")
        }
        _ => (0, 0, 0, ""),
    };

    let line = Line::from(vec![
        Span::raw("  "),
        Span::styled("Success: ", Style::default().fg(Color::Green)),
        Span::styled(
            success.to_string(),
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("   "),
        Span::styled("Failed: ", Style::default().fg(Color::Red)),
        Span::styled(
            failed.to_string(),
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        ),
        Span::raw("   "),
        Span::styled("Skipped: ", Style::default().fg(Color::DarkGray)),
        Span::styled(skipped.to_string(), Style::default().fg(Color::DarkGray)),
        Span::raw("   "),
        Span::styled(current, Style::default().fg(Color::Cyan)),
    ]);

    let counters = Paragraph::new(vec![Line::from(""), line]);
    frame.render_widget(counters, area);
}

fn render_log(app: &App, frame: &mut Frame, area: ratatui::layout::Rect) {
    let visible_height = area.height.saturating_sub(2) as usize; // account for borders
    let total = app.log_lines.len();
    let start = total.saturating_sub(visible_height);

    let items: Vec<ListItem> = app.log_lines[start..]
        .iter()
        .map(|line| {
            let style = if line.starts_with("[ok]") {
                Style::default().fg(Color::Green)
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
