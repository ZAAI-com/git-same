//! Settings screen — two-pane layout with category nav (left) and detail (right).

use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::tui::app::App;
use crate::tui::widgets::status_bar;

const CATEGORIES: &[&str] = &["Folders", "Options"];

pub fn render(app: &App, frame: &mut Frame) {
    let chunks = Layout::vertical([
        Constraint::Length(3), // Title
        Constraint::Min(5),    // Content (two panes)
        Constraint::Length(1), // Status bar
    ])
    .split(frame.area());

    // Title
    let title = Paragraph::new(Line::from(vec![Span::styled(
        " Settings ",
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    )
    .centered();
    frame.render_widget(title, chunks[0]);

    // Two-pane split
    let panes =
        Layout::horizontal([Constraint::Percentage(25), Constraint::Percentage(75)])
            .split(chunks[1]);

    render_category_nav(app, frame, panes[0]);

    match app.settings_index {
        0 => render_folders_detail(app, frame, panes[1]),
        1 => render_options_detail(app, frame, panes[1]),
        _ => {}
    }

    // Status bar — context-sensitive hints
    let hint = match app.settings_index {
        0 => {
            let ws_hint = if app.workspaces.is_empty() {
                String::new()
            } else {
                let max = app.workspaces.len().min(9);
                format!("  1-{}: Open workspace", max)
            };
            format!(
                "Tab: Switch  j/k: Nav  c: Config{}  Esc: Back  q: Quit",
                ws_hint
            )
        }
        1 => "Tab: Switch  j/k: Nav  d: Dry-run  m: Mode  Esc: Back  q: Quit".to_string(),
        _ => "Esc: Back  q: Quit".to_string(),
    };
    status_bar::render(frame, chunks[2], &hint);
}

fn render_category_nav(app: &App, frame: &mut Frame, area: Rect) {
    let items: Vec<ListItem> = CATEGORIES
        .iter()
        .enumerate()
        .map(|(i, name)| {
            let marker = if i == app.settings_index { ">" } else { " " };
            let style = if i == app.settings_index {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(Line::from(vec![
                Span::styled(format!(" {} ", marker), style),
                Span::styled(*name, style),
            ]))
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    frame.render_widget(list, area);
}

fn render_folders_detail(app: &App, frame: &mut Frame, area: Rect) {
    let dim = Style::default().fg(Color::DarkGray);
    let key_style = Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD);
    let section_style = Style::default()
        .fg(Color::White)
        .add_modifier(Modifier::BOLD);
    let active_style = Style::default()
        .fg(Color::Green)
        .add_modifier(Modifier::BOLD);

    let config_path = crate::config::Config::default_path()
        .ok()
        .and_then(|p| p.parent().map(|parent| parent.display().to_string()))
        .unwrap_or_else(|| "~/.config/git-same".to_string());

    let mut lines = vec![
        Line::from(""),
        Line::from(Span::styled("  Open Folders", section_style)),
        Line::from(""),
        Line::from(vec![
            Span::styled("    ", dim),
            Span::styled("[c]", key_style),
            Span::styled("  Config folder", dim),
            Span::styled(format!("  — {}", config_path), dim),
        ]),
    ];

    if app.workspaces.is_empty() {
        lines.push(Line::from(Span::styled(
            "    (no workspaces configured)",
            dim,
        )));
    } else {
        for (i, ws) in app.workspaces.iter().enumerate() {
            if i >= 9 {
                break;
            }
            let is_active = app
                .active_workspace
                .as_ref()
                .map(|active| active.name == ws.name)
                .unwrap_or(false);

            let mut spans = vec![
                Span::styled("    ", dim),
                Span::styled(format!("[{}]", i + 1), key_style),
                Span::styled(format!("  {}", ws.name), dim),
                Span::styled(format!("  — {}", ws.base_path), dim),
            ];
            if is_active {
                spans.push(Span::styled("  (active)", active_style));
            }
            lines.push(Line::from(spans));
        }
    }

    let content = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    frame.render_widget(content, area);
}

fn render_options_detail(app: &App, frame: &mut Frame, area: Rect) {
    let dim = Style::default().fg(Color::DarkGray);
    let key_style = Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD);
    let section_style = Style::default()
        .fg(Color::White)
        .add_modifier(Modifier::BOLD);
    let active_style = Style::default()
        .fg(Color::Green)
        .add_modifier(Modifier::BOLD);

    // Dry run toggle
    let (dry_yes, dry_no) = if app.dry_run {
        (active_style, dim)
    } else {
        (dim, active_style)
    };

    // Mode toggle
    let (mode_fetch, mode_pull) = if app.sync_pull {
        (dim, active_style)
    } else {
        (active_style, dim)
    };

    let lines = vec![
        Line::from(""),
        Line::from(Span::styled("  Global Config", section_style)),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                format!("    Concurrency: {}", app.config.concurrency),
                dim,
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled("  Options", section_style)),
        Line::from(""),
        Line::from(vec![
            Span::styled("    ", dim),
            Span::styled("[d]", key_style),
            Span::styled("  Dry run:  ", dim),
            Span::styled("Yes", dry_yes),
            Span::styled(" / ", dim),
            Span::styled("No", dry_no),
        ]),
        Line::from(vec![
            Span::styled("    ", dim),
            Span::styled("[m]", key_style),
            Span::styled("  Mode:     ", dim),
            Span::styled("Fetch", mode_fetch),
            Span::styled(" / ", dim),
            Span::styled("Pull", mode_pull),
        ]),
    ];

    let content = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    frame.render_widget(content, area);
}
