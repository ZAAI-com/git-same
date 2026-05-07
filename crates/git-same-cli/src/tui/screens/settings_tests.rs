use super::*;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use git_same_core::config::{Config, WorkspaceConfig};
use ratatui::backend::TestBackend;
use ratatui::Terminal;

fn render_output(app: &App) -> String {
    let backend = TestBackend::new(110, 32);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal.draw(|frame| render(app, frame)).unwrap();

    let buffer = terminal.backend().buffer();
    let mut text = String::new();
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            text.push_str(buffer[(x, y)].symbol());
        }
        text.push('\n');
    }
    text
}

fn app_for_settings() -> App {
    let ws = WorkspaceConfig::new_from_root(std::path::Path::new("/tmp/ws"));
    App::new(Config::default(), vec![ws], false)
}

#[test]
fn handle_key_moves_selection_and_toggles_flags() {
    let mut app = app_for_settings();
    app.settings_index = 0;

    handle_key(&mut app, KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    assert_eq!(app.settings_index, 1);

    handle_key(&mut app, KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    assert_eq!(app.settings_index, 0);

    handle_key(&mut app, KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
    assert_eq!(app.settings_index, 1);

    assert!(!app.dry_run);
    handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE),
    );
    assert!(app.dry_run);

    assert!(!app.sync_pull);
    handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Char('m'), KeyModifiers::NONE),
    );
    assert!(app.sync_pull);
}

#[test]
fn render_requirements_view_shows_title_and_loading() {
    let mut app = app_for_settings();
    app.settings_index = 0;
    app.checks_loading = true;

    let output = render_output(&app);
    assert!(output.contains("Settings"));
    assert!(output.contains("Requirements"));
    assert!(output.contains("Loading"));
}

#[test]
fn render_options_view_shows_mode_and_dry_run() {
    let mut app = app_for_settings();
    app.settings_index = 1;
    app.dry_run = true;
    app.sync_pull = true;

    let output = render_output(&app);
    assert!(output.contains("Global Config"));
    assert!(output.contains("Dry run"));
    assert!(output.contains("Mode"));
    assert!(output.contains("Fetch"));
    assert!(output.contains("Pull"));
}
