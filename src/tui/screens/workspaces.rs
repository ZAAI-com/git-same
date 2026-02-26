//! Workspace screen — two-pane layout with workspace list (left) and detail (right).
//!
//! Left sidebar lists all workspaces plus a "Create New Workspace" entry.
//! Right panel shows detail for the selected workspace or a create prompt.

use chrono::{DateTime, Utc};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crossterm::event::{KeyCode, KeyEvent};
use tokio::sync::mpsc::UnboundedSender;

#[cfg(test)]
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::banner::render_banner;
use crate::config::{Config, SyncMode, WorkspaceConfig, WorkspaceManager};
use crate::setup::state::SetupState;
use crate::tui::app::{App, Screen, WorkspacePane};
use crate::tui::event::{AppEvent, BackendMessage};

#[cfg(test)]
static OPEN_WORKSPACE_FOLDER_CALLS: AtomicUsize = AtomicUsize::new(0);

// ── Key handler ─────────────────────────────────────────────────────────────

pub async fn handle_key(app: &mut App, key: KeyEvent, backend_tx: &UnboundedSender<AppEvent>) {
    let num_ws = app.workspaces.len();
    let total_entries = num_ws + 1; // workspaces + "Create New Workspace"

    match key.code {
        KeyCode::Left => {
            app.workspace_pane = WorkspacePane::Left;
        }
        KeyCode::Right => {
            app.workspace_pane = WorkspacePane::Right;
        }
        KeyCode::Tab => {
            app.workspace_pane = match app.workspace_pane {
                WorkspacePane::Left => WorkspacePane::Right,
                WorkspacePane::Right => WorkspacePane::Left,
            };
        }
        // Right pane: scroll detail when config is expanded
        KeyCode::Down
            if app.workspace_pane == WorkspacePane::Right && app.settings_config_expanded =>
        {
            app.workspace_detail_scroll = app.workspace_detail_scroll.saturating_add(1);
        }
        KeyCode::Up
            if app.workspace_pane == WorkspacePane::Right && app.settings_config_expanded =>
        {
            app.workspace_detail_scroll = app.workspace_detail_scroll.saturating_sub(1);
        }
        // Left pane: navigate workspace list
        KeyCode::Down if app.workspace_pane == WorkspacePane::Left && total_entries > 0 => {
            app.workspace_index = (app.workspace_index + 1) % total_entries;
            app.settings_config_expanded = false;
            app.workspace_detail_scroll = 0;
        }
        KeyCode::Up if app.workspace_pane == WorkspacePane::Left && total_entries > 0 => {
            app.workspace_index = (app.workspace_index + total_entries - 1) % total_entries;
            app.settings_config_expanded = false;
            app.workspace_detail_scroll = 0;
        }
        KeyCode::Enter => {
            if app.workspace_index < num_ws {
                // Select workspace and go to dashboard
                app.select_workspace(app.workspace_index);
                app.screen = Screen::Dashboard;
                app.screen_stack.clear();
            } else {
                // "Create New Workspace" entry
                let default_path = std::env::current_dir()
                    .map(|p| crate::setup::state::tilde_collapse(&p.to_string_lossy()))
                    .unwrap_or_else(|_| "~/Git-Same/GitHub".to_string());
                app.setup_state = Some(SetupState::new(&default_path));
                app.navigate_to(Screen::WorkspaceSetup);
            }
        }
        KeyCode::Char('c') if app.workspace_index < num_ws => {
            app.workspace_pane = WorkspacePane::Right;
            app.settings_config_expanded = !app.settings_config_expanded;
            app.workspace_detail_scroll = 0;
        }
        KeyCode::Char('n') => {
            // Shortcut to create workspace
            let default_path = std::env::current_dir()
                .map(|p| crate::setup::state::tilde_collapse(&p.to_string_lossy()))
                .unwrap_or_else(|_| "~/Git-Same/GitHub".to_string());
            app.setup_state = Some(SetupState::new(&default_path));
            app.navigate_to(Screen::WorkspaceSetup);
        }
        KeyCode::Char('d') if app.workspace_index < num_ws => {
            // Set default workspace
            if let Some(ws) = app.workspaces.get(app.workspace_index) {
                let ws_name = ws.name.clone();
                let new_default_name = match next_default_workspace_name(
                    app.config.default_workspace.as_deref(),
                    &ws_name,
                ) {
                    Some(name) => name,
                    None => {
                        return;
                    }
                };

                let new_default = Some(new_default_name);
                let tx = backend_tx.clone();
                let default_clone = new_default.clone();
                tokio::spawn(async move {
                    match Config::save_default_workspace(default_clone.as_deref()) {
                        Ok(()) => {
                            let _ = tx.send(AppEvent::Backend(
                                BackendMessage::DefaultWorkspaceUpdated(default_clone),
                            ));
                        }
                        Err(e) => {
                            let _ = tx.send(AppEvent::Backend(
                                BackendMessage::DefaultWorkspaceError(format!("{}", e)),
                            ));
                        }
                    }
                });
            }
        }
        KeyCode::Char('f') if app.workspace_index < num_ws => {
            // Open workspace folder
            if let Some(ws) = app.workspaces.get(app.workspace_index) {
                let path = ws.expanded_base_path();
                open_workspace_folder(&path);
            }
        }
        _ => {}
    }
}

fn next_default_workspace_name(
    current_default: Option<&str>,
    selected_workspace: &str,
) -> Option<String> {
    if current_default == Some(selected_workspace) {
        None
    } else {
        Some(selected_workspace.to_string())
    }
}

#[cfg(not(test))]
fn open_workspace_folder(path: &std::path::Path) {
    let _ = std::process::Command::new("open").arg(path).spawn();
}

#[cfg(test)]
fn open_workspace_folder(_path: &std::path::Path) {
    OPEN_WORKSPACE_FOLDER_CALLS.fetch_add(1, Ordering::SeqCst);
}

#[cfg(test)]
fn take_open_workspace_folder_call_count() -> usize {
    OPEN_WORKSPACE_FOLDER_CALLS.swap(0, Ordering::SeqCst)
}

// ── Render ──────────────────────────────────────────────────────────────────

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

    if !app.workspaces.is_empty() {
        items.push(ListItem::new(Line::from("")));
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

    // "Create New Workspace" entry
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
        Span::styled("Create New Workspace [n]", create_style),
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
    let key_style = Style::default()
        .fg(Color::Rgb(37, 99, 235))
        .add_modifier(Modifier::BOLD);

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

    let sync_mode = ws
        .sync_mode
        .map(sync_mode_name)
        .map(ToString::to_string)
        .unwrap_or_else(|| format!("{} (global default)", sync_mode_name(app.config.sync_mode)));

    let concurrency = ws
        .concurrency
        .map(|c| c.to_string())
        .unwrap_or_else(|| format!("{} (global default)", app.config.concurrency));

    let (last_synced_relative, last_synced_absolute) =
        format_last_synced(ws.last_synced.as_deref());

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

    lines.push(detail_row_with_hint(
        area,
        "Active",
        active_label,
        Some(("[Enter]", "Select Workspace")),
        dim,
        val_style,
        key_style,
    ));
    lines.push(detail_row_with_hint(
        area,
        "Default",
        default_label,
        if is_default {
            None
        } else {
            Some(("[d]", "Set default"))
        },
        dim,
        val_style,
        key_style,
    ));
    lines.push(Line::from(vec![
        Span::styled(format!("    {:<14}", "Provider"), dim),
        Span::styled(ws.provider.kind.display_name().to_string(), val_style),
    ]));

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled("  Paths", section_style)));
    lines.push(Line::from(""));
    lines.push(detail_row_with_hint(
        area,
        "Path",
        &ws.base_path,
        None,
        dim,
        val_style,
        key_style,
    ));
    lines.push(detail_row_with_hint(
        area,
        "Full path",
        &full_path,
        Some(("[f]", "Open Finder Folder")),
        dim,
        val_style,
        key_style,
    ));
    lines.push(detail_row_with_hint(
        area,
        "Config",
        &config_file,
        None,
        dim,
        val_style,
        key_style,
    ));
    lines.push(Line::from(vec![
        Span::styled(format!("    {:<14}", "Cache file"), dim),
        Span::styled(cache_file, val_style),
    ]));

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled("  Sync", section_style)));
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled(format!("    {:<14}", "Sync mode"), dim),
        Span::styled(sync_mode, val_style),
    ]));
    lines.push(Line::from(vec![
        Span::styled(format!("    {:<14}", "Concurrency"), dim),
        Span::styled(concurrency, val_style),
    ]));
    lines.push(Line::from(vec![
        Span::styled(format!("    {:<14}", "Last synced"), dim),
        Span::styled(last_synced_relative, val_style),
    ]));
    if let Some(absolute) = last_synced_absolute {
        lines.push(Line::from(vec![
            Span::styled(format!("    {:<14}", ""), dim),
            Span::styled(absolute, val_style),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled("  Account", section_style)));
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled(format!("    {:<14}", "Username"), dim),
        Span::styled(username, val_style),
    ]));
    let org_lines = wrap_comma_separated_values(&ws.orgs, field_value_width(area, 14));
    if let Some((first, rest)) = org_lines.split_first() {
        lines.push(Line::from(vec![
            Span::styled(format!("    {:<14}", "Organizations"), dim),
            Span::styled(first.as_str(), val_style),
        ]));
        for line in rest {
            lines.push(Line::from(vec![
                Span::styled(format!("    {:<14}", ""), dim),
                Span::styled(line.as_str(), val_style),
            ]));
        }
    }

    // Config content section (collapsible)
    lines.push(Line::from(""));
    if app.settings_config_expanded {
        lines.push(section_line_with_hint(
            area,
            "\u{25BC} Config",
            "[c]",
            "Collapse config file",
            section_style,
            dim,
            key_style,
        ));
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
        lines.push(section_line_with_hint(
            area,
            "\u{25B6} Config",
            "[c]",
            "Expand config file",
            section_style,
            dim,
            key_style,
        ));
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

fn sync_mode_name(mode: SyncMode) -> &'static str {
    match mode {
        SyncMode::Fetch => "fetch",
        SyncMode::Pull => "pull",
    }
}

fn detail_row_with_hint(
    area: Rect,
    label: &str,
    value: &str,
    hint: Option<(&str, &str)>,
    dim: Style,
    val_style: Style,
    key_style: Style,
) -> Line<'static> {
    let right_padding = 2usize;
    let label_text = format!("    {:<14}", label);
    let mut spans = vec![
        Span::styled(label_text.clone(), dim),
        Span::styled(value.to_string(), val_style),
    ];

    if let Some((hint_key, hint_label)) = hint {
        let content_width = area.width.saturating_sub(2) as usize;
        let left_width = label_text.chars().count() + value.chars().count();
        let hint_width = hint_key.chars().count() + 1 + hint_label.chars().count() + right_padding;
        let gap = content_width.saturating_sub(left_width + hint_width).max(1);

        spans.push(Span::raw(" ".repeat(gap)));
        spans.push(Span::styled(hint_key.to_string(), key_style));
        spans.push(Span::styled(format!(" {}", hint_label), dim));
        spans.push(Span::raw(" ".repeat(right_padding)));
    }

    Line::from(spans)
}

fn section_line_with_hint(
    area: Rect,
    section: &str,
    hint_key: &str,
    hint_label: &str,
    section_style: Style,
    dim: Style,
    key_style: Style,
) -> Line<'static> {
    let right_padding = 2usize;
    let section_text = format!("  {}", section);
    let content_width = area.width.saturating_sub(2) as usize;
    let left_width = section_text.chars().count();
    let hint_width = hint_key.chars().count() + 1 + hint_label.chars().count() + right_padding;
    let gap = content_width.saturating_sub(left_width + hint_width).max(1);

    Line::from(vec![
        Span::styled(section_text, section_style),
        Span::raw(" ".repeat(gap)),
        Span::styled(hint_key.to_string(), key_style),
        Span::styled(format!(" {}", hint_label), dim),
        Span::raw(" ".repeat(right_padding)),
    ])
}

fn format_last_synced(raw: Option<&str>) -> (String, Option<String>) {
    let Some(raw) = raw else {
        return ("never".to_string(), None);
    };

    match DateTime::parse_from_rfc3339(raw) {
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
            (relative, Some(absolute))
        }
        Err(_) => (raw.to_string(), None),
    }
}

fn field_value_width(area: Rect, label_width: usize) -> usize {
    let content_width = area.width.saturating_sub(2) as usize;
    let prefix_width = 4 + label_width;
    content_width.saturating_sub(prefix_width).max(16)
}

fn wrap_comma_separated_values(values: &[String], max_width: usize) -> Vec<String> {
    if values.is_empty() {
        return vec!["all".to_string()];
    }

    let mut lines = Vec::new();
    let mut current = String::new();

    for value in values {
        if current.is_empty() {
            current.push_str(value);
            continue;
        }

        if current.len() + 2 + value.len() <= max_width {
            current.push_str(", ");
            current.push_str(value);
        } else {
            lines.push(current);
            current = value.clone();
        }
    }

    if !current.is_empty() {
        lines.push(current);
    }

    lines
}

fn render_create_workspace_detail(frame: &mut Frame, area: Rect) {
    let dim = Style::default().fg(Color::DarkGray);
    let section_style = Style::default()
        .fg(Color::White)
        .add_modifier(Modifier::BOLD);
    let key_style = Style::default()
        .fg(Color::Rgb(37, 99, 235))
        .add_modifier(Modifier::BOLD);

    let lines = vec![
        Line::from(""),
        section_line_with_hint(
            area,
            "New Workspace",
            "[n]",
            "Create New Workspace",
            section_style,
            dim,
            key_style,
        ),
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

    // Line 1: intentionally blank (action hints are shown inline in the right panel)
    let actions = Paragraph::new(vec![Line::from("")]).centered();

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

    let right_spans = if app.workspace_pane == WorkspacePane::Right
        && app.workspace_index < app.workspaces.len()
        && app.settings_config_expanded
    {
        vec![
            Span::styled("[\u{2190}]", key_style),
            Span::raw(" "),
            Span::styled("[\u{2192}]", key_style),
            Span::styled(" Panel", dim),
            Span::raw("   "),
            Span::styled("[\u{2191}]", key_style),
            Span::raw(" "),
            Span::styled("[\u{2193}]", key_style),
            Span::styled(" Scroll", dim),
            Span::raw("   "),
            Span::styled("[c]", key_style),
            Span::styled(" Collapse", dim),
            Span::raw("  "),
        ]
    } else if app.workspace_pane == WorkspacePane::Left {
        vec![
            Span::styled("[\u{2190}]", key_style),
            Span::raw(" "),
            Span::styled("[\u{2192}]", key_style),
            Span::styled(" Panel", dim),
            Span::raw("   "),
            Span::styled("[\u{2191}]", key_style),
            Span::raw(" "),
            Span::styled("[\u{2193}]", key_style),
            Span::styled(" Move", dim),
            Span::raw("   "),
            Span::styled("[Enter]", key_style),
            Span::styled(" Select", dim),
            Span::raw("  "),
        ]
    } else {
        vec![
            Span::styled("[\u{2190}]", key_style),
            Span::raw(" "),
            Span::styled("[\u{2192}]", key_style),
            Span::styled(" Panel", dim),
            Span::raw("   "),
            Span::styled("[c]", key_style),
            Span::styled(" Expand", dim),
            Span::raw("   "),
            Span::styled("[Enter]", key_style),
            Span::styled(" Select", dim),
            Span::raw("  "),
        ]
    };

    frame.render_widget(actions, rows[0]);
    frame.render_widget(Paragraph::new(vec![Line::from(left_spans)]), nav_cols[0]);
    frame.render_widget(
        Paragraph::new(vec![Line::from(right_spans)]).right_aligned(),
        nav_cols[1],
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use tokio::sync::mpsc::error::TryRecvError;

    #[test]
    fn wrap_comma_separated_values_wraps_and_preserves_order() {
        let values = vec![
            "CommitBook".to_string(),
            "GenAI-Wednesday".to_string(),
            "M-com".to_string(),
            "Manuel-Forks".to_string(),
        ];

        let lines = wrap_comma_separated_values(&values, 20);
        assert!(lines.len() > 1);
        assert_eq!(lines.join(", "), values.join(", "));
    }

    #[test]
    fn wrap_comma_separated_values_empty_means_all() {
        let lines = wrap_comma_separated_values(&[], 20);
        assert_eq!(lines, vec!["all".to_string()]);
    }

    fn build_workspace_app(default_workspace: Option<&str>) -> App {
        let mut config = Config::default();
        config.default_workspace = default_workspace.map(ToString::to_string);

        let ws = WorkspaceConfig::new("test-ws", "/tmp/test-ws");
        let mut app = App::new(config, vec![ws.clone()]);
        app.screen = Screen::Workspaces;
        app.workspace_index = 0;
        app.active_workspace = Some(ws);
        app
    }

    #[tokio::test]
    async fn workspace_key_f_opens_folder_for_selected_workspace() {
        let mut app = build_workspace_app(None);
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let _ = take_open_workspace_folder_call_count();

        handle_key(
            &mut app,
            KeyEvent::new(KeyCode::Char('f'), KeyModifiers::NONE),
            &tx,
        )
        .await;

        assert_eq!(take_open_workspace_folder_call_count(), 1);
    }

    #[tokio::test]
    async fn workspace_key_c_toggles_config_expansion() {
        let mut app = build_workspace_app(None);
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

        handle_key(
            &mut app,
            KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE),
            &tx,
        )
        .await;

        assert_eq!(app.workspace_pane, WorkspacePane::Right);
        assert!(app.settings_config_expanded);

        handle_key(
            &mut app,
            KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE),
            &tx,
        )
        .await;

        assert!(!app.settings_config_expanded);
        assert!(matches!(rx.try_recv(), Err(TryRecvError::Empty)));
    }

    #[tokio::test]
    async fn workspace_left_right_controls_panel_focus_and_list_movement() {
        let mut config = Config::default();
        config.default_workspace = None;
        let ws1 = WorkspaceConfig::new("ws1", "/tmp/ws1");
        let ws2 = WorkspaceConfig::new("ws2", "/tmp/ws2");
        let mut app = App::new(config, vec![ws1.clone(), ws2]);
        app.screen = Screen::Workspaces;
        app.workspace_index = 0;
        app.active_workspace = Some(ws1);
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

        handle_key(
            &mut app,
            KeyEvent::new(KeyCode::Right, KeyModifiers::NONE),
            &tx,
        )
        .await;
        assert_eq!(app.workspace_pane, WorkspacePane::Right);

        handle_key(
            &mut app,
            KeyEvent::new(KeyCode::Down, KeyModifiers::NONE),
            &tx,
        )
        .await;
        assert_eq!(app.workspace_index, 0);

        handle_key(
            &mut app,
            KeyEvent::new(KeyCode::Left, KeyModifiers::NONE),
            &tx,
        )
        .await;
        assert_eq!(app.workspace_pane, WorkspacePane::Left);

        handle_key(
            &mut app,
            KeyEvent::new(KeyCode::Down, KeyModifiers::NONE),
            &tx,
        )
        .await;
        assert_eq!(app.workspace_index, 1);
        assert!(matches!(rx.try_recv(), Err(TryRecvError::Empty)));
    }

    #[tokio::test]
    async fn workspace_key_o_is_noop() {
        let mut app = build_workspace_app(None);
        let before_index = app.workspace_index;
        let before_scroll = app.workspace_detail_scroll;
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let _ = take_open_workspace_folder_call_count();

        handle_key(
            &mut app,
            KeyEvent::new(KeyCode::Char('o'), KeyModifiers::NONE),
            &tx,
        )
        .await;

        assert_eq!(app.workspace_index, before_index);
        assert_eq!(app.workspace_detail_scroll, before_scroll);
        assert_eq!(take_open_workspace_folder_call_count(), 0);
        assert!(matches!(rx.try_recv(), Err(TryRecvError::Empty)));
    }

    #[tokio::test]
    async fn workspace_enter_selects_workspace_even_if_active() {
        let mut app = build_workspace_app(None);
        app.settings_config_expanded = true;
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

        handle_key(
            &mut app,
            KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
            &tx,
        )
        .await;

        assert_eq!(app.screen, Screen::Dashboard);
        assert!(matches!(rx.try_recv(), Err(TryRecvError::Empty)));
    }

    #[tokio::test]
    async fn workspace_key_d_does_not_clear_when_already_default() {
        let mut app = build_workspace_app(Some("test-ws"));
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

        handle_key(
            &mut app,
            KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE),
            &tx,
        )
        .await;

        assert_eq!(app.config.default_workspace.as_deref(), Some("test-ws"));
        assert!(matches!(rx.try_recv(), Err(TryRecvError::Empty)));
    }

    #[test]
    fn next_default_workspace_name_is_set_only() {
        assert_eq!(
            next_default_workspace_name(Some("current"), "next"),
            Some("next".to_string())
        );
        assert_eq!(next_default_workspace_name(Some("same"), "same"), None);
        assert_eq!(
            next_default_workspace_name(None, "selected"),
            Some("selected".to_string())
        );
    }
}
