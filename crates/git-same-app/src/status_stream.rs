//! Watches the monitor's IPC directory and pushes changes to the UI.
//!
//! Two independent signals come out of the same directory:
//! * `status.json` changed: new badge data (`status-updated`).
//! * the runtime identity record or `status.json` changed: the monitor
//!   started, finished its first scan, or exited, including when that was
//!   triggered from a terminal (`monitor-agent-updated`).
//!
//! Temp files of atomic writes, the lock file, and caches are ignored.

use crate::commands::{read_status_snapshot_with, refresh_monitor_status};
use git_same_core::ipc::IpcConfig;
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

/// Bursts (temp write, rename, record update) collapse into one update.
const DEBOUNCE: Duration = Duration::from_millis(400);

/// Upper bound on how long a continuous stream of writes can hold an update
/// back. Without it a trailing-edge debounce can starve the UI indefinitely.
const MAX_DEBOUNCE: Duration = Duration::from_millis(2000);

/// What a changed file in the IPC directory means for the UI.
///
/// Ordered: folding a multi-path event and merging a burst both take the
/// most significant relevance seen.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Relevance {
    #[default]
    Ignore,
    /// Monitor lifecycle only.
    Monitor,
    /// New badge data, which is also the starting-to-running transition.
    DataAndMonitor,
}

/// The two files worth reacting to, matched by name within the watched
/// directory.
///
/// Byte-exact `PathBuf` comparison is not enough: `notify`'s FSEvents backend
/// reports resolved paths (`/private/var/...`) while `IpcConfig.dir` comes
/// from the passwd home uncanonicalised, so on a Mac with a symlinked or
/// relocated home every event would be ignored and the UI would silently
/// never receive a push update.
pub(crate) struct WatchTargets {
    /// The watched directory in every form an event may name it.
    dirs: Vec<PathBuf>,
    status_name: OsString,
    identity_name: OsString,
}

impl WatchTargets {
    pub(crate) fn new(ipc: &IpcConfig) -> Self {
        let mut dirs = vec![ipc.dir.clone()];
        if let Ok(resolved) = ipc.dir.canonicalize() {
            if resolved != ipc.dir {
                dirs.push(resolved);
            }
        }
        Self {
            dirs,
            status_name: file_name_of(&ipc.status_file_path()),
            identity_name: file_name_of(&ipc.runtime_identity_path()),
        }
    }

    pub(crate) fn relevance(&self, path: &Path) -> Relevance {
        let (Some(name), Some(parent)) = (path.file_name(), path.parent()) else {
            return Relevance::Ignore;
        };
        if !self.is_watched_dir(parent) {
            return Relevance::Ignore;
        }
        if name == self.status_name {
            Relevance::DataAndMonitor
        } else if name == self.identity_name {
            Relevance::Monitor
        } else {
            Relevance::Ignore
        }
    }

    fn is_watched_dir(&self, dir: &Path) -> bool {
        if self.dirs.iter().any(|known| known == dir) {
            return true;
        }
        // Only reached for a path spelled differently from both known forms,
        // so the syscall is not on the common path.
        dir.canonicalize()
            .is_ok_and(|resolved| self.dirs.contains(&resolved))
    }
}

fn file_name_of(path: &Path) -> OsString {
    path.file_name().unwrap_or(path.as_os_str()).to_os_string()
}

/// Collects a burst of events into one update.
///
/// Trailing edge: each further event pushes the deadline out, so a scan no
/// longer emits `status-updated` per filesystem event. `MAX_DEBOUNCE` caps
/// how far it can be pushed.
#[derive(Default)]
pub(crate) struct Debouncer {
    due: Option<Instant>,
    first: Option<Instant>,
    pending: Relevance,
}

impl Debouncer {
    /// Records an event. `now` is injected so the logic is testable.
    pub(crate) fn record(&mut self, relevance: Relevance, now: Instant) {
        if relevance == Relevance::Ignore {
            return;
        }
        self.pending = self.pending.max(relevance);
        let first = *self.first.get_or_insert(now);
        let capped = first + MAX_DEBOUNCE;
        self.due = Some((now + DEBOUNCE).min(capped));
    }

    /// How long to wait for the next event.
    pub(crate) fn timeout(&self, now: Instant) -> Duration {
        self.due
            .map(|due| due.saturating_duration_since(now))
            .unwrap_or(Duration::from_secs(3600))
    }

    /// Takes the collected relevance if the deadline has passed.
    pub(crate) fn take_due(&mut self, now: Instant) -> Option<Relevance> {
        let due = self.due?;
        if now < due {
            return None;
        }
        self.due = None;
        self.first = None;
        Some(std::mem::take(&mut self.pending))
    }
}

/// `ipc` is the resolved host-facing config (`~/.config/git-same/finder/`,
/// where the monitor mirrors a real `status.json`), so neither the watch nor
/// the reads cross into the app-group container.
pub fn spawn_watcher(app: AppHandle, ipc: IpcConfig) -> anyhow::Result<()> {
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

            // Resolved once: the directory exists by now (`ensure_dir`), and
            // an event's spelling does not change during the session.
            let targets = WatchTargets::new(&ipc);
            let mut debouncer = Debouncer::default();

            loop {
                let timeout = debouncer.timeout(Instant::now());
                match rx.recv_timeout(timeout) {
                    Ok(Ok(event)) => {
                        let relevance = event
                            .paths
                            .iter()
                            .map(|path| targets.relevance(path))
                            .max()
                            .unwrap_or(Relevance::Ignore);
                        debouncer.record(relevance, Instant::now());
                    }
                    Ok(Err(_)) => {}
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                        let Some(fired) = debouncer.take_due(Instant::now()) else {
                            continue;
                        };
                        if fired == Relevance::DataAndMonitor {
                            if let Ok(snapshot) = read_status_snapshot_with(&ipc) {
                                let _ = app.emit("status-updated", snapshot);
                            }
                        }
                        if fired != Relevance::Ignore {
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
