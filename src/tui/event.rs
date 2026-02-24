//! Event system: merges terminal input and backend notifications.

use crossterm::event::{self, Event as CtEvent, KeyEvent};
use std::time::Duration;
use tokio::sync::mpsc;

use crate::types::{OpSummary, OwnedRepo};

use super::app::{CheckEntry, Operation, RepoEntry};

/// Events that the TUI loop processes.
#[derive(Debug)]
pub enum AppEvent {
    /// A keyboard event from the terminal.
    Terminal(KeyEvent),
    /// Terminal resize.
    Resize(u16, u16),
    /// Backend sent a progress update.
    Backend(BackendMessage),
    /// Periodic tick for animations/spinners.
    Tick,
}

/// Messages from backend async operations.
#[derive(Debug, Clone)]
pub enum BackendMessage {
    /// Discovery: orgs found.
    OrgsDiscovered(usize),
    /// Discovery: processing an org.
    OrgStarted(String),
    /// Discovery: org complete with N repos.
    OrgComplete(String, usize),
    /// Discovery complete with full repo list.
    DiscoveryComplete(Vec<OwnedRepo>),
    /// Discovery failed.
    DiscoveryError(String),
    /// Operation phase started with N total repos.
    OperationStarted { operation: Operation, total: usize },
    /// Operation progress: one repo processed.
    RepoProgress {
        repo_name: String,
        success: bool,
        skipped: bool,
        message: String,
    },
    /// Operation complete.
    OperationComplete(OpSummary),
    /// Operation error.
    OperationError(String),
    /// Status scan results.
    StatusResults(Vec<RepoEntry>),
    /// Init: config file created successfully.
    InitConfigCreated(String),
    /// Init: config creation failed.
    InitConfigError(String),
    /// Default workspace was set/cleared successfully.
    DefaultWorkspaceUpdated(Option<String>),
    /// Default workspace operation failed.
    DefaultWorkspaceError(String),
    /// Requirement check results (background).
    CheckResults(Vec<CheckEntry>),
}

/// Spawn the terminal event reader in a blocking thread.
/// Returns a receiver for AppEvents and a sender for backend to push messages.
pub fn spawn_event_loop(
    tick_rate: Duration,
) -> (
    mpsc::UnboundedReceiver<AppEvent>,
    mpsc::UnboundedSender<AppEvent>,
) {
    let (tx, rx) = mpsc::unbounded_channel();
    let event_tx = tx.clone();

    // Terminal event reader (crossterm is blocking)
    tokio::task::spawn_blocking(move || {
        loop {
            if event::poll(tick_rate).unwrap_or(false) {
                if let Ok(ev) = event::read() {
                    let app_event = match ev {
                        CtEvent::Key(key) => AppEvent::Terminal(key),
                        CtEvent::Resize(w, h) => AppEvent::Resize(w, h),
                        _ => continue,
                    };
                    if event_tx.send(app_event).is_err() {
                        break;
                    }
                }
            } else {
                // Tick on timeout
                if event_tx.send(AppEvent::Tick).is_err() {
                    break;
                }
            }
        }
    });

    (rx, tx)
}
