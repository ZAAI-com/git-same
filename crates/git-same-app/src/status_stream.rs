//! Watches the monitor's IPC directory and pushes changes to the UI.
//!
//! Two independent signals come out of the same directory:
//! * `status.json` changed: new badge data (`status-updated`).
//! * the runtime identity record or `status.json` changed: the monitor
//!   started, finished its first scan, or exited, including when that was
//!   triggered from a terminal (`monitor-agent-updated`).
//!
//! Temp files of atomic writes, the lock file, and caches are ignored.

use crate::commands::{read_status_snapshot, refresh_monitor_status};
use git_same_core::ipc::IpcConfig;
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::Path;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

/// Bursts (temp write, rename, record update) collapse into one inspect.
const MONITOR_DEBOUNCE: Duration = Duration::from_millis(400);

/// What a changed file in the IPC directory means for the UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Relevance {
    Ignore,
    /// Monitor lifecycle only.
    Monitor,
    /// New badge data, which is also the starting-to-running transition.
    DataAndMonitor,
}

pub(crate) fn relevance(path: &Path, ipc: &IpcConfig) -> Relevance {
    if path == ipc.status_file_path() {
        Relevance::DataAndMonitor
    } else if path == ipc.runtime_identity_path() {
        Relevance::Monitor
    } else {
        Relevance::Ignore
    }
}

pub fn spawn_watcher(app: AppHandle) -> anyhow::Result<()> {
    let ipc = IpcConfig::default_path()?;
    ipc.ensure_dir()?;

    std::thread::Builder::new()
        .name("git-same-status-watcher".to_string())
        .spawn(move || {
            let (tx, rx) = std::sync::mpsc::channel();
            let mut watcher = match RecommendedWatcher::new(tx, Config::default()) {
                Ok(watcher) => watcher,
                Err(error) => {
                    eprintln!("failed to create status watcher: {error}");
                    return;
                }
            };

            if let Err(error) = watcher.watch(&ipc.dir, RecursiveMode::NonRecursive) {
                eprintln!(
                    "failed to watch status directory '{}': {error}",
                    ipc.dir.display()
                );
                return;
            }

            let mut monitor_due: Option<Instant> = None;
            loop {
                let timeout = monitor_due
                    .map(|due| due.saturating_duration_since(Instant::now()))
                    .unwrap_or(Duration::from_secs(3600));
                match rx.recv_timeout(timeout) {
                    Ok(Ok(event)) => {
                        let relevance = event.paths.iter().map(|path| relevance(path, &ipc)).fold(
                            Relevance::Ignore,
                            |most, next| match (most, next) {
                                (Relevance::DataAndMonitor, _) | (_, Relevance::DataAndMonitor) => {
                                    Relevance::DataAndMonitor
                                }
                                (Relevance::Monitor, _) | (_, Relevance::Monitor) => {
                                    Relevance::Monitor
                                }
                                _ => Relevance::Ignore,
                            },
                        );
                        if relevance == Relevance::DataAndMonitor {
                            if let Ok(snapshot) = read_status_snapshot() {
                                let _ = app.emit("status-updated", snapshot);
                            }
                        }
                        if relevance != Relevance::Ignore && monitor_due.is_none() {
                            monitor_due = Some(Instant::now() + MONITOR_DEBOUNCE);
                        }
                    }
                    Ok(Err(_)) => {}
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                        if monitor_due.take().is_some() {
                            // Inspecting runs launchctl: never on this thread.
                            refresh_monitor_status(app.clone());
                        }
                    }
                    Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => return,
                }
            }
        })?;

    Ok(())
}

#[cfg(test)]
#[path = "status_stream_tests.rs"]
mod tests;
