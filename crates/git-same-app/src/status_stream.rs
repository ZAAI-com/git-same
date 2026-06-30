use crate::commands::read_status_snapshot;
use git_same_core::ipc::IpcConfig;
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use tauri::{AppHandle, Emitter};

pub fn spawn_watcher(app: AppHandle) -> anyhow::Result<()> {
    let ipc = IpcConfig::default_path()?;
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
                if event.is_err() {
                    continue;
                }
                if let Ok(snapshot) = read_status_snapshot() {
                    let _ = app.emit("status-updated", snapshot);
                }
            }
        })?;

    Ok(())
}
