//! Interactive setup wizard for creating workspace configurations.
//!
//! This module provides a self-contained ratatui mini-app that guides
//! the user through setting up a workspace: selecting a provider,
//! authenticating, selecting organizations, and choosing a base path.

pub mod handler;
pub mod screens;
pub mod state;
pub mod ui;

use crate::errors::Result;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, Event as CtEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use state::{SetupOutcome, SetupState, SetupStep};
use std::io;
use std::time::Duration;

/// Run the setup wizard.
///
/// Returns `Ok(true)` if the wizard completed (workspace saved),
/// `Ok(false)` if the user cancelled.
pub async fn run_setup() -> Result<bool> {
    let default_path = std::env::current_dir()
        .map(|p| state::tilde_collapse(&p.to_string_lossy()))
        .unwrap_or_else(|_| "~/Git-Same/GitHub".to_string());
    let mut state = SetupState::new(&default_path);

    struct SetupTerminalGuard {
        raw_enabled: bool,
        alt_enabled: bool,
    }
    impl Drop for SetupTerminalGuard {
        fn drop(&mut self) {
            if self.alt_enabled {
                let mut stdout = io::stdout();
                let _ = execute!(stdout, LeaveAlternateScreen, DisableMouseCapture);
            }
            if self.raw_enabled {
                let _ = disable_raw_mode();
            }
        }
    }

    // Setup terminal
    enable_raw_mode()?;
    let mut guard = SetupTerminalGuard {
        raw_enabled: true,
        alt_enabled: false,
    };
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    guard.alt_enabled = true;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Main loop
    let result = run_wizard(&mut terminal, &mut state).await;

    // Restore terminal (always, even on error)
    let _ = disable_raw_mode();
    guard.raw_enabled = false;
    let _ = execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    );
    guard.alt_enabled = false;
    let _ = terminal.show_cursor();

    result?;

    Ok(matches!(state.outcome, Some(SetupOutcome::Completed)))
}

async fn run_wizard(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    state: &mut SetupState,
) -> Result<()> {
    loop {
        terminal.draw(|frame| ui::render(state, frame))?;

        // If we're on the orgs step and loading, trigger discovery before waiting for input
        if state.step == SetupStep::SelectOrgs && state.org_loading {
            // Render loading state first, then do discovery
            terminal.draw(|frame| ui::render(state, frame))?;
            handler::handle_key(
                state,
                crossterm::event::KeyEvent::new(
                    crossterm::event::KeyCode::Null,
                    crossterm::event::KeyModifiers::NONE,
                ),
            )
            .await;
            continue;
        }

        // Increment tick counter for animations
        state.tick_count = state.tick_count.wrapping_add(1);

        // Wait for input with a short timeout for responsive tick
        if crossterm::event::poll(Duration::from_millis(100))? {
            if let Ok(event) = crossterm::event::read() {
                match event {
                    CtEvent::Key(key) => {
                        handler::handle_key(state, key).await;
                    }
                    CtEvent::Resize(_, _) => {
                        // Terminal will re-render on next loop iteration
                    }
                    _ => {}
                }
            }
        }

        if state.should_quit {
            break;
        }
    }
    Ok(())
}
