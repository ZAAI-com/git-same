//! Workspace screen — two-pane layout with workspace list (left) and detail (right).
//!
//! Left sidebar lists all workspaces plus a "Create Workspace" entry.
//! Right panel shows detail for the selected workspace or a create prompt.

use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::banner::render_banner;
use crate::config::{WorkspaceConfig, WorkspaceManager};
use crate::tui::app::App;
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
        " Workspaces ",
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

    render_workspace_nav(app, frame, panes[0]);

    if app.workspace_index < app.workspaces.len() {
        if let Some(ws) = app.workspaces.get(app.workspace_index) {
            render_workspace_detail(app, ws, frame, panes[1]);
        }
    } else {
        render_create_workspace_detail(frame, panes[1]);
    }

    render_bottom_actions(app, frame, chunks[3]);
}

fn render_workspace_nav(app: &App, frame: &mut Frame, area: Rect) {
    let dim = Style::default().fg(Color::DarkGray);
    let mut items: Vec<ListItem> = Vec::new();

    if app.workspaces.is_empty() {
        items.push(ListItem::new(Line::from(Span::styled(
            "    (no workspaces)",
            dim,
        ))));
    }

    for (i, ws) in app.workspaces.iter().enumerate() {
        let selected = app.workspace_index == i;
        let is_active = app
            .active_workspace
            .as_ref()
            .map(|aw| aw.name == ws.name)
            .unwrap_or(false);
        let is_default = app.config.default_workspace.as_deref() == Some(ws.name.as_str());

        let folder_name = std::path::Path::new(&ws.base_path)
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or(&ws.base_path);

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

        let mut spans = vec![
            Span::styled(format!("  {} ", marker), style),
            Span::styled(folder_name.to_string(), style),
        ];

        if is_active {
            spans.push(Span::styled(
                " \u{25CF}",
                Style::default()
                    .fg(Color::Rgb(21, 128, 61))
                    .add_modifier(Modifier::BOLD),
            ));
        }
        if is_default {
            spans.push(Span::styled(
                " (default)",
                Style::default().fg(Color::Rgb(21, 128, 61)),
            ));
        }

        items.push(ListItem::new(Line::from(spans)));
    }

    // Spacer before Create entry
    if !app.workspaces.is_empty() {
        items.push(ListItem::new(Line::from("")));
    }

    // "Create Workspace" entry
    let create_selected = app.workspace_index == app.workspaces.len();
    let (create_marker, create_style) = if create_selected {
        (
            ">",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        (" ", Style::default().fg(Color::Rgb(21, 128, 61)))
    };
    items.push(ListItem::new(Line::from(vec![
        Span::styled(format!("  {} ", create_marker), create_style),
        Span::styled("+ Create Workspace", create_style),
    ])));

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    frame.render_widget(list, area);
}

fn render_workspace_detail(app: &App, ws: &WorkspaceConfig, frame: &mut Frame, area: Rect) {
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

    let is_active = app
        .active_workspace
        .as_ref()
        .map(|aw| aw.name == ws.name)
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
    let active_label = if is_active { "Yes" } else { "No" };

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
        ("Active", active_label.to_string()),
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

    let content = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        )
        .scroll((app.workspace_detail_scroll, 0));
    frame.render_widget(content, area);
}

fn render_create_workspace_detail(frame: &mut Frame, area: Rect) {
    let dim = Style::default().fg(Color::DarkGray);
    let section_style = Style::default()
        .fg(Color::White)
        .add_modifier(Modifier::BOLD);

    let lines = vec![
        Line::from(""),
        Line::from(Span::styled("  Create Workspace", section_style)),
        Line::from(""),
        Line::from(Span::styled(
            "    Press Enter to launch the Setup Wizard",
            dim,
        )),
        Line::from(Span::styled("    and configure a new workspace.", dim)),
        Line::from(""),
        Line::from(Span::styled("    The wizard will guide you through:", dim)),
        Line::from(Span::styled(
            "      \u{2022} Choosing a base directory",
            dim,
        )),
        Line::from(Span::styled(
            "      \u{2022} Connecting to a provider (GitHub)",
            dim,
        )),
        Line::from(Span::styled(
            "      \u{2022} Selecting organizations to sync",
            dim,
        )),
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
    if app.workspace_index < app.workspaces.len() {
        // Workspace selected
        action_spans.extend([
            Span::raw(" "),
            Span::styled("[Enter]", key_style),
            Span::styled(" Switch / Config", dim),
            Span::raw("   "),
            Span::styled("[d]", key_style),
            Span::styled(" Default", dim),
            Span::raw("   "),
            Span::styled("[o]", key_style),
            Span::styled(" Open folder", dim),
            Span::raw("   "),
            Span::styled("[n]", key_style),
            Span::styled(" New", dim),
        ]);
    } else {
        // "Create Workspace" selected
        action_spans.extend([
            Span::raw(" "),
            Span::styled("[Enter]", key_style),
            Span::styled(" Create workspace", dim),
        ]);
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
