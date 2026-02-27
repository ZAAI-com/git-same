//! Event system: merges terminal input and backend notifications.

use crossterm::event::{self, Event as CtEvent, KeyEvent};
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::warn;

use crate::setup::state::OrgEntry;
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
    /// Setup wizard org discovery complete.
    SetupOrgsDiscovered(Vec<OrgEntry>),
    /// Setup wizard org discovery failed.
    SetupOrgsError(String),
    /// Operation phase started with total and per-phase breakdown.
    OperationStarted {
        operation: Operation,
        total: usize,
        to_clone: usize,
        to_sync: usize,
    },
    /// A repo started processing (for live worker slots).
    RepoStarted { repo_name: String },
    /// Operation progress: one repo processed.
    RepoProgress {
        repo_name: String,
        success: bool,
        skipped: bool,
        message: String,
        /// Whether this repo had new commits.
        had_updates: bool,
        /// Whether this was a clone (not a sync).
        is_clone: bool,
        /// Number of new commits fetched (if known).
        new_commits: Option<u32>,
        /// Structured skip reason (if skipped).
        skip_reason: Option<String>,
    },
    /// Commit log for a specific repo (post-sync deep dive).
    RepoCommitLog {
        repo_name: String,
        commits: Vec<String>,
    },
    /// Operation complete.
    OperationComplete(OpSummary),
    /// Operation error.
    OperationError(String),
    /// Status scan results.
    StatusResults(Vec<RepoEntry>),
    /// Setup wizard requirement check results.
    SetupCheckResults(Vec<CheckEntry>),
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
            let has_event = match event::poll(tick_rate) {
                Ok(v) => v,
                Err(e) => {
                    warn!(error = %e, "Terminal poll failed; stopping event loop");
                    break;
                }
            };

            if has_event {
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

#[cfg(test)]
#[path = "event_tests.rs"]
mod tests;
