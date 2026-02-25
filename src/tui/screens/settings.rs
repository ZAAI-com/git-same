//! Settings screen — two-pane layout with hierarchical nav (left) and detail (right).
//!
//! Left sidebar groups: "Global" (Requirements, Options) and "Workspaces" (one per workspace).
//! Right panel shows detail for the selected item.

use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::config::WorkspaceManager;
use crate::tui::app::App;
use crate::banner::render_banner;
use crate::tui::screens::dashboard::format_timestamp;

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
        i if i >= 2 => {
            let ws_idx = i - 2;
            if let Some(ws) = app.workspaces.get(ws_idx) {
                render_workspace_detail(app, ws, frame, panes[1]);
            }
        }
        _ => {}
    }

    render_bottom_actions(app, frame, chunks[3]);
}

fn render_category_nav(app: &App, frame: &mut Frame, area: Rect) {
    let header_style = Style::default()
        .fg(Color::White)
        .add_modifier(Modifier::BOLD);
    let dim = Style::default().fg(Color::DarkGray);

    let mut items: Vec<ListItem> = vec![
        // -- Global header --
        ListItem::new(Line::from(Span::styled("  Global", header_style))),
        // Requirements (index 0)
        nav_item("Requirements", app.settings_index == 0),
        // Options (index 1)
        nav_item("Options", app.settings_index == 1),
        // Spacer
        ListItem::new(Line::from(Span::styled("", dim))),
        // -- Workspaces header --
        ListItem::new(Line::from(Span::styled("  Workspaces", header_style))),
    ];

    // Each workspace (show folder name, i.e. last path component)
    for (i, ws) in app.workspaces.iter().enumerate() {
        let selected = app.settings_index == 2 + i;
        let folder_name = std::path::Path::new(&ws.base_path)
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or(&ws.base_path);
        items.push(nav_item(folder_name, selected));
    }

    if app.workspaces.is_empty() {
        items.push(ListItem::new(Line::from(Span::styled("    (none)", dim))));
    }

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
        .fg(Color::Green)
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

fn render_workspace_detail(
    app: &App,
    ws: &crate::config::WorkspaceConfig,
    frame: &mut Frame,
    area: Rect,
) {
    let dim = Style::default().fg(Color::DarkGray);
    let section_style = Style::default()
        .fg(Color::White)
        .add_modifier(Modifier::BOLD);
    let val_style = Style::default().fg(Color::White);

    let is_default = app
        .config
        .default_workspace
        .as_deref()
        .map(|d| d == ws.name)
        .unwrap_or(false);

    let full_path = ws.expanded_base_path().display().to_string();

    let config_file = WorkspaceManager::workspace_dir(&ws.name)
        .map(|d| d.join("workspace-config.toml").display().to_string())
        .unwrap_or_else(|_| "unknown".to_string());

    let cache_file = WorkspaceManager::cache_path(&ws.name)
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "unknown".to_string());

    let username = if ws.username.is_empty() {
        "\u{2014}".to_string()
    } else {
        ws.username.clone()
    };

    let orgs = if ws.orgs.is_empty() {
        "all".to_string()
    } else {
        ws.orgs.join(", ")
    };

    let sync_mode = ws
        .sync_mode
        .as_ref()
        .map(|m| format!("{:?}", m))
        .unwrap_or_else(|| "global default".to_string());

    let concurrency = ws
        .concurrency
        .map(|c| c.to_string())
        .unwrap_or_else(|| format!("{} (global)", app.config.concurrency));

    let last_synced = ws
        .last_synced
        .as_deref()
        .map(format_timestamp)
        .unwrap_or_else(|| "never".to_string());

    let default_label = if is_default { "Yes" } else { "No" };

    let folder_name = std::path::Path::new(&ws.base_path)
        .file_name()
        .and_then(|f| f.to_str())
        .unwrap_or(&ws.base_path);

    let mut lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("  Workspace: {}", folder_name),
            section_style,
        )),
        Line::from(""),
    ];

    let fields: Vec<(&str, String)> = vec![
        ("Path", ws.base_path.clone()),
        ("Provider", ws.provider.kind.display_name().to_string()),
        ("Default", default_label.to_string()),
        ("Full path", full_path),
        ("Config file", config_file),
        ("Cache file", cache_file),
        ("Username", username),
        ("Organizations", orgs),
        ("Sync mode", sync_mode),
        ("Concurrency", concurrency),
        ("Last synced", last_synced),
    ];

    for (label, value) in &fields {
        lines.push(Line::from(vec![
            Span::styled(format!("    {:<14}", label), dim),
            Span::styled(value.as_str(), val_style),
        ]));
    }

    // Config content section (collapsible)
    lines.push(Line::from(""));
    if app.settings_config_expanded {
        lines.push(Line::from(Span::styled("  \u{25BC} Config", section_style)));
        lines.push(Line::from(""));
        match ws.to_toml() {
            Ok(toml) => {
                for toml_line in toml.lines() {
                    lines.push(Line::from(Span::styled(format!("    {}", toml_line), dim)));
                }
            }
            Err(_) => {
                lines.push(Line::from(Span::styled(
                    "    (failed to serialize config)",
                    dim,
                )));
            }
        }
    } else {
        lines.push(Line::from(vec![
            Span::styled("  \u{25B6} Config", section_style),
            Span::styled("  (press Enter to expand)", dim),
        ]));
    }

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
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD);

    // Line 1: Context-sensitive actions (centered)
    let mut action_spans = vec![];
    match app.settings_index {
        1 => {
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
        i if i >= 2 => {
            action_spans.extend([
                Span::raw(" "),
                Span::styled("[Enter]", key_style),
                Span::styled(" Config", dim),
                Span::raw("   "),
                Span::styled("[o]", key_style),
                Span::styled(" Open folder", dim),
            ]);
        }
        _ => {}
    }
    let actions = Paragraph::new(vec![Line::from(action_spans)]).centered();

    // Line 2: Navigation — left (quit, back) and right (arrows)
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
        Span::styled("[Tab]", key_style),
        Span::styled(" Switch", dim),
        Span::raw("   "),
        Span::styled("[\u{2191}]", key_style),
        Span::raw(" "),
        Span::styled("[\u{2193}]", key_style),
        Span::styled(" Move", dim),
        Span::raw("   "),
        Span::styled("[Enter]", key_style),
        Span::styled(" Select", dim),
        Span::raw(" "),
    ];

    frame.render_widget(actions, rows[0]);
    frame.render_widget(Paragraph::new(vec![Line::from(left_spans)]), nav_cols[0]);
    frame.render_widget(
        Paragraph::new(vec![Line::from(right_spans)]).right_aligned(),
        nav_cols[1],
    );
}
