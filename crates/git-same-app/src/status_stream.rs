use crate::commands::read_status_snapshot_with;
use git_same_core::ipc::IpcConfig;
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::ffi::OsStr;
use tauri::{AppHandle, Emitter};

/// Watches the host-facing IPC directory for `status.json` changes and emits a
/// `status-updated` event with a fresh snapshot. `ipc` is the resolved
/// host-facing config (`~/.config/git-same/finder/`, where the monitor mirrors
/// a real `status.json`), so neither the watch nor the reads cross into the
/// app-group container.
pub fn spawn_watcher(app: AppHandle, ipc: IpcConfig) -> anyhow::Result<()> {
    ipc.ensure_dir()?;
    let watch_path = ipc.dir.clone();

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

            if let Err(error) = watcher.watch(&watch_path, RecursiveMode::NonRecursive) {
                eprintln!(
                    "failed to watch status directory '{}': {error}",
                    watch_path.display()
                );
                return;
            }

            for event in rx {
                let event = match event {
                    Ok(event) => event,
                    Err(error) => {
                        eprintln!("status watcher event error: {error}");
                        continue;
                    }
                };
                if !event_touches_status_file(&event) {
                    continue;
                }
                match read_status_snapshot_with(&ipc) {
                    Ok(snapshot) => {
                        let _ = app.emit("status-updated", snapshot);
                    }
                    Err(error) => {
                        eprintln!("failed to read status snapshot: {error}");
                    }
                }
            }
        })?;

    Ok(())
}

/// Whether a watcher event concerns the final `status.json` rather than, for
/// example, the sibling `status.json.tmp` the atomic write creates first.
/// Without this filter every monitor write (tmp create + rename) triggers
/// several full snapshot reads and duplicate `status-updated` emits.
///
/// Rescan events are always kept: after a kernel-side queue drop, notify's
/// macOS FSEvents backend marks the event for rescan but still attaches the
/// watched-directory path. Pathless events are also kept defensively because
/// other backends may use them to signal that an update was missed.
fn event_touches_status_file(event: &Event) -> bool {
    event.need_rescan()
        || event.paths.is_empty()
        || event
            .paths
            .iter()
            .any(|path| path.file_name() == Some(OsStr::new("status.json")))
}

#[cfg(test)]
#[path = "status_stream_tests.rs"]
mod tests;
