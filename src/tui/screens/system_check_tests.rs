use super::*;
use crate::config::{Config, WorkspaceConfig};
use crate::tui::app::Screen;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::backend::TestBackend;
use ratatui::Terminal;
use tokio::sync::mpsc::unbounded_channel;

fn render_output(app: &App) -> String {
    let backend = TestBackend::new(110, 28);
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

fn app_for_screen() -> App {
    let ws = WorkspaceConfig::new_from_root(std::path::Path::new("/tmp/ws"));
    App::new(Config::default(), vec![ws])
}

#[tokio::test]
async fn handle_key_s_opens_setup_wizard() {
    let mut app = app_for_screen();
    app.screen = Screen::SystemCheck;

    let (tx, _rx) = unbounded_channel();
    handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE),
        &tx,
    )
    .await;

    assert_eq!(app.screen, Screen::WorkspaceSetup);
    assert!(app.setup_state.is_some());
}

#[test]
fn render_loading_state_shows_checking_message() {
    let mut app = app_for_screen();
    app.checks_loading = true;
    app.check_results.clear();

    let output = render_output(&app);
    assert!(output.contains("System Requirements"));
    assert!(output.contains("Checking requirements"));
}

#[test]
fn render_results_state_shows_create_config_hint() {
    let mut app = app_for_screen();
    app.checks_loading = false;
    app.config_created = false;
    app.check_results = vec![CheckEntry {
        name: "git".to_string(),
        passed: true,
        message: "installed".to_string(),
        critical: true,
    }];

    let output = render_output(&app);
    assert!(output.contains("Results"));
    assert!(output.contains("Press 'c' to create config"));
}
