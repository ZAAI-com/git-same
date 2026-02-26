//! Settings screen — two-pane layout with nav (left) and detail (right).
//!
//! Left sidebar: "Global" section with Requirements and Options.
//! Right panel shows detail for the selected item.

use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crossterm::event::{KeyCode, KeyEvent};

use crate::banner::render_banner;
use crate::tui::app::App;

pub fn handle_key(app: &mut App, key: KeyEvent) {
    let num_items = 2; // Requirements, Options
    match key.code {
        KeyCode::Tab | KeyCode::Down => {
            app.settings_index = (app.settings_index + 1) % num_items;
        }
        KeyCode::Up => {
            app.settings_index = (app.settings_index + num_items - 1) % num_items;
        }
        KeyCode::Char('c') => {
            // Open config directory in Finder / file manager
            if let Ok(path) = crate::config::Config::default_path() {
                if let Some(parent) = path.parent() {
                    if let Err(e) = open_directory(parent) {
                        app.error_message = Some(format!(
                            "Failed to open config directory '{}': {}",
                            parent.display(),
                            e
                        ));
                    }
                }
            }
        }
        KeyCode::Char('d') => {
            app.dry_run = !app.dry_run;
        }
        KeyCode::Char('m') => {
            app.sync_pull = !app.sync_pull;
        }
        _ => {}
    }
}

#[cfg(target_os = "macos")]
fn open_directory(path: &std::path::Path) -> std::io::Result<()> {
    std::process::Command::new("open").arg(path).spawn()?;
    Ok(())
}

#[cfg(target_os = "windows")]
fn open_directory(path: &std::path::Path) -> std::io::Result<()> {
    std::process::Command::new("explorer").arg(path).spawn()?;
    Ok(())
}

#[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
fn open_directory(path: &std::path::Path) -> std::io::Result<()> {
    std::process::Command::new("xdg-open").arg(path).spawn()?;
    Ok(())
}

pub fn render(app: &App, frame: &mut Frame) {
    let chunks = Layout::vertical([
        Constraint::Length(6), // Banner
        Constraint::Length(3), // Title
        Constraint::Min(5),    // Content (two panes)
        Constraint::Length(2), // Bottom actions (2 lines)
    ])
    .split(frame.area());

    render_banner(frame, chunks[0]);

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
    frame.render_widget(title, chunks[1]);

    // Two-pane split
    let panes = Layout::horizontal([Constraint::Percentage(25), Constraint::Percentage(75)])
        .split(chunks[2]);

    render_category_nav(app, frame, panes[0]);

    match app.settings_index {
        0 => render_requirements_detail(app, frame, panes[1]),
        1 => render_options_detail(app, frame, panes[1]),
        _ => render_requirements_detail(app, frame, panes[1]),
    }

    render_bottom_actions(app, frame, chunks[3]);
}

fn render_category_nav(app: &App, frame: &mut Frame, area: Rect) {
    let header_style = Style::default()
        .fg(Color::White)
        .add_modifier(Modifier::BOLD);

    let items: Vec<ListItem> = vec![
        // -- Global header --
        ListItem::new(Line::from(Span::styled("  Global", header_style))),
        // Requirements (index 0)
        nav_item("Requirements", app.settings_index == 0),
        // Options (index 1)
        nav_item("Options", app.settings_index == 1),
    ];

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    frame.render_widget(list, area);
}

fn nav_item(label: &str, selected: bool) -> ListItem<'static> {
    let (marker, style) = if selected {
        (
            ">",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        (" ", Style::default())
    };
    ListItem::new(Line::from(vec![
        Span::styled(format!("  {} ", marker), style),
        Span::styled(label.to_string(), style),
    ]))
}

fn render_requirements_detail(app: &App, frame: &mut Frame, area: Rect) {
    let dim = Style::default().fg(Color::DarkGray);
    let section_style = Style::default()
        .fg(Color::White)
        .add_modifier(Modifier::BOLD);
    let pass_style = Style::default()
        .fg(Color::Rgb(21, 128, 61))
        .add_modifier(Modifier::BOLD);
    let fail_style = Style::default().fg(Color::Red).add_modifier(Modifier::BOLD);

    let mut lines = vec![
        Line::from(""),
        Line::from(Span::styled("  Requirements", section_style)),
        Line::from(""),
    ];

    if app.check_results.is_empty() {
        let msg = if app.checks_loading {
            "    Loading..."
        } else {
            "    Checks not yet run"
        };
        lines.push(Line::from(Span::styled(msg, dim)));
    } else {
        for check in &app.check_results {
            let (marker, marker_style) = if check.passed {
                ("\u{2713}", pass_style)
            } else {
                ("\u{2717}", fail_style)
            };
            let mut spans = vec![
                Span::styled("    ", dim),
                Span::styled(marker.to_string(), marker_style),
                Span::styled(format!("  {:<14}", check.name), dim),
                Span::styled(&check.message, dim),
            ];
            if !check.passed && check.critical {
                spans.push(Span::styled("  (critical)", fail_style));
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
        .fg(Color::Rgb(37, 99, 235))
        .add_modifier(Modifier::BOLD);
    let section_style = Style::default()
        .fg(Color::White)
        .add_modifier(Modifier::BOLD);
    let active_style = Style::default()
        .fg(Color::Rgb(21, 128, 61))
        .add_modifier(Modifier::BOLD);

    let config_path = crate::config::Config::default_path()
        .ok()
        .and_then(|p| p.parent().map(|parent| parent.display().to_string()))
        .unwrap_or_else(|| "~/.config/git-same".to_string());

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
        Line::from(vec![Span::styled(
            format!("    Concurrency: {}", app.config.concurrency),
            dim,
        )]),
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
        Line::from(""),
        Line::from(Span::styled("  Folders", section_style)),
        Line::from(""),
        Line::from(vec![
            Span::styled("    ", dim),
            Span::styled("[c]", key_style),
            Span::styled("  Config folder", dim),
            Span::styled(format!("  \u{2014} {}", config_path), dim),
        ]),
    ];

    let content = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );
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
        .fg(Color::Rgb(37, 99, 235))
        .add_modifier(Modifier::BOLD);

    // Line 1: Context-sensitive actions (centered)
    let mut action_spans = vec![];
    if app.settings_index == 1 {
        action_spans.extend([
            Span::raw(" "),
            Span::styled("[c]", key_style),
            Span::styled(" Config", dim),
            Span::raw("   "),
            Span::styled("[d]", key_style),
            Span::styled(" Dry-run", dim),
            Span::raw("   "),
            Span::styled("[m]", key_style),
            Span::styled(" Mode", dim),
        ]);
    }
    let actions = Paragraph::new(vec![Line::from(action_spans)]).centered();

    // Line 2: Navigation — left (quit, back) and right (arrows)
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
        Span::styled("[Tab]", key_style),
        Span::styled(" Switch", dim),
        Span::raw("   "),
        Span::styled("[\u{2191}]", key_style),
        Span::raw(" "),
        Span::styled("[\u{2193}]", key_style),
        Span::styled(" Move", dim),
        Span::raw(" "),
    ];

    frame.render_widget(actions, rows[0]);
    frame.render_widget(Paragraph::new(vec![Line::from(left_spans)]), nav_cols[0]);
    frame.render_widget(
        Paragraph::new(vec![Line::from(right_spans)]).right_aligned(),
        nav_cols[1],
    );
}

#[cfg(test)]
#[path = "settings_tests.rs"]
mod tests;
