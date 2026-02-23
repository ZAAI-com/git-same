//! Full-screen TUI for git-same.
//!
//! Launched when `gisa` is run with no subcommand.

pub mod app;
pub mod backend;
pub mod event;
pub mod handler;
pub mod screens;
pub mod ui;
pub mod widgets;

use crate::config::Config;
use crate::errors::Result;
use app::App;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io;
use std::time::Duration;

/// Run the TUI application.
pub async fn run_tui(config: Config) -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app state
    let mut app = App::new(config);

    // Start event loop
    let tick_rate = Duration::from_millis(100);
    let (mut rx, backend_tx) = event::spawn_event_loop(tick_rate);

    // Main loop
    let result = run_app(&mut terminal, &mut app, &mut rx, &backend_tx).await;

    // Restore terminal (always, even on error)
    let _ = disable_raw_mode();
    let _ = execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    );
    let _ = terminal.show_cursor();

    result
}

async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    rx: &mut tokio::sync::mpsc::UnboundedReceiver<event::AppEvent>,
    backend_tx: &tokio::sync::mpsc::UnboundedSender<event::AppEvent>,
) -> Result<()> {
    loop {
        terminal.draw(|frame| ui::render(app, frame))?;

        if let Some(event) = rx.recv().await {
            handler::handle_event(app, event, backend_tx).await;
        }

        if app.should_quit {
            break;
        }
    }
    Ok(())
}
