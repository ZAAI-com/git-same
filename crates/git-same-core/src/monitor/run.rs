//! Monitor loop entry point.
//!
//! Event-driven scans (notify FSEvents + socket REFRESH) are the primary
//! source of updates; a periodic full `scan_all` is kept as a safety net
//! for events `notify` may have dropped and for ambient repos that appear
//! in scan roots without a parent we are subscribed to. The full-scan
//! cadence is controlled by `Options::interval` (in turn driven by the CLI
//! `--interval` flag and `config.monitor.fullscan_interval_secs`).

use crate::api::{AmbientUpgradeCache, OwnerTypeCache, RepoScanService};
use crate::config::Config;
use crate::errors::Result;
use crate::git::ShellGit;
use crate::ipc::status_file::ensure_legacy_symlinks;
use crate::ipc::{IpcConfig, StatusFileWriter};
use crate::monitor::incremental::rescan_and_merge;
use crate::output::Output;
use crate::types::FinderStatus;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashSet;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tracing::{debug, error, info, warn};

use super::owner_classifier::spawn_owner_classifier;

/// Trailing-edge debounce window for collapsing FSEvent bursts (e.g. a
/// single `git commit` fires many .git/ writes) into one repo rescan.
const FS_EVENT_DEBOUNCE: Duration = Duration::from_millis(750);

/// Options for [`run`].
#[derive(Debug, Clone)]
pub struct Options {
    /// Cadence of the safety-net full `scan_all`. Most updates flow through
    /// the FSEvents arm; this timer covers dropped events and catches
    /// ambient repos that appear without a parent we subscribed to.
    pub interval: Duration,
    /// Resolved IPC paths (status file + socket).
    pub ipc_config: IpcConfig,
}

/// Run the monitor loop until `shutdown` resolves.
pub async fn run<S>(config: &Config, output: &Output, opts: Options, shutdown: S) -> Result<()>
where
    S: Future<Output = ()>,
{
    let Options {
        interval,
        ipc_config,
    } = opts;

    ipc_config.ensure_dir()?;

    if let Err(e) = ensure_legacy_symlinks(&ipc_config.dir) {
        warn!("Could not refresh legacy IPC symlinks: {}", e);
    }

    info!("Starting git-same monitor");
    output.info("Starting git-same monitor...");

    let primary_status_path = ipc_config.status_file_path();
    let status_writer = StatusFileWriter::new_with_mirrors(
        primary_status_path.clone(),
        status_mirror_paths(&primary_status_path),
    );
    let git = ShellGit::new();

    let owner_types = OwnerTypeCache::load(OwnerTypeCache::default_path(&ipc_config.dir));
    let ambient_upgrades = AmbientUpgradeCache::new();
    let service = RepoScanService::new(&git, config)
        .with_owner_types(owner_types.clone())
        .with_ambient_upgrades(ambient_upgrades.clone());
    spawn_owner_classifier(config.clone(), owner_types);

    let pid = std::process::id();

    let initial_status = service.scan_all(pid)?;
    status_writer.write(&initial_status)?;
    let ambient_count = initial_status
        .repos
        .iter()
        .filter(|r| r.workspace.is_none())
        .count();
    let workspace_count = initial_status.repos.len() - ambient_count;
    info!(
        repos = initial_status.repos.len(),
        workspace = workspace_count,
        ambient = ambient_count,
        "Initial scan complete, status written"
    );
    output.info(&format!(
        "Monitoring {} repos ({} workspace, {} ambient). Status: {}",
        initial_status.repos.len(),
        workspace_count,
        ambient_count,
        ipc_config.status_file_path().display()
    ));

    let watched_roots = collect_watched_roots(config, &initial_status);
    reapply_workspace_folder_icons(config, &initial_status);
    let shared_status = Arc::new(Mutex::new(initial_status));

    #[cfg(unix)]
    let socket_listener = crate::ipc::UnixSocketListener::new(ipc_config.socket_path());
    #[cfg(unix)]
    let tokio_listener = socket_listener.bind().await?;

    let (fs_tx, mut fs_rx) = tokio::sync::mpsc::unbounded_channel::<PathBuf>();
    let _watcher = match start_filesystem_watcher(&watched_roots, fs_tx) {
        Ok(watcher) => Some(watcher),
        Err(e) => {
            warn!(error = %e, "Filesystem watcher failed to start; monitor will only respond to REFRESH commands");
            None
        }
    };

    let mut pending: HashSet<PathBuf> = HashSet::new();

    tokio::pin!(shutdown);

    loop {
        let debounce_active = !pending.is_empty();

        #[cfg(unix)]
        {
            tokio::select! {
                _ = tokio::time::sleep(FS_EVENT_DEBOUNCE), if debounce_active => {
                    flush_pending(&service, &shared_status, &status_writer, &ambient_upgrades, &mut pending);
                },
                _ = tokio::time::sleep(interval) => {
                    debug!("Safety-net full scan");
                    match service.scan_all(pid) {
                        Ok(new_status) => {
                            reapply_workspace_folder_icons(config, &new_status);
                            let mut status = shared_status.lock().expect("status mutex poisoned");
                            *status = new_status;
                            if let Err(e) = status_writer.write(&status) {
                                error!(error = %e, "Failed to write status file after full scan");
                            } else {
                                debug!(repos = status.repos.len(), "Full scan complete");
                            }
                        }
                        Err(e) => {
                            error!(error = %e, "Full scan failed");
                        }
                    }
                },
                Some(repo_path) = fs_rx.recv() => {
                    pending.insert(repo_path);
                },
                result = tokio_listener.accept() => {
                    match result {
                        Ok((stream, _)) => {
                            let config_clone = config.clone();
                            let writer = status_writer.clone();
                            let owner_clone = service.owner_types_clone();
                            let ambient_clone = service.ambient_upgrades_clone();
                            let status_clone = shared_status.clone();
                            tokio::spawn(async move {
                                super::socket_handler::handle_socket_connection(
                                    stream,
                                    &config_clone,
                                    pid,
                                    writer,
                                    status_clone,
                                    owner_clone,
                                    ambient_clone,
                                ).await;
                            });
                        }
                        Err(e) => {
                            warn!(error = %e, "Failed to accept socket connection");
                        }
                    }
                },
                _ = &mut shutdown => {
                    info!("Monitor shutting down");
                    output.info("Monitor shutting down...");
                    socket_listener.cleanup();
                    break;
                },
            }
        }

        #[cfg(not(unix))]
        {
            tokio::select! {
                _ = tokio::time::sleep(FS_EVENT_DEBOUNCE), if debounce_active => {
                    flush_pending(&service, &shared_status, &status_writer, &ambient_upgrades, &mut pending);
                },
                _ = tokio::time::sleep(interval) => {
                    debug!("Safety-net full scan");
                    if let Ok(new_status) = service.scan_all(pid) {
                        reapply_workspace_folder_icons(config, &new_status);
                        let mut status = shared_status.lock().expect("status mutex poisoned");
                        *status = new_status;
                        let _ = status_writer.write(&status);
                    }
                },
                Some(repo_path) = fs_rx.recv() => {
                    pending.insert(repo_path);
                },
                _ = &mut shutdown => {
                    info!("Monitor shutting down");
                    output.info("Monitor shutting down...");
                    break;
                },
            }
        }
    }

    let _ = ambient_upgrades;
    Ok(())
}

/// Mirror paths for the status writer. On macOS the primary `status.json`
/// lives in the app-group container; mirror a real copy into the host-facing
/// `~/.config/git-same/finder/` so the non-sandboxed Tauri host can read live
/// status without reaching into the container (which would trigger the "access
/// data from other apps" TCC prompt). On other platforms the primary path is
/// already the host path, so there are no mirrors.
fn status_mirror_paths(primary: &Path) -> Vec<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        if let Ok(host) = IpcConfig::host_status_path() {
            let mirror = host.status_file_path();
            if mirror.as_path() != primary {
                return vec![mirror];
            }
        }
        Vec::new()
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = primary;
        Vec::new()
    }
}

fn flush_pending(
    service: &RepoScanService<'_>,
    shared_status: &Arc<Mutex<FinderStatus>>,
    status_writer: &StatusFileWriter,
    ambient_upgrades: &AmbientUpgradeCache,
    pending: &mut HashSet<PathBuf>,
) {
    if pending.is_empty() {
        return;
    }
    let mut any_changed = false;
    let mut status = shared_status.lock().expect("status mutex poisoned");
    for repo in pending.drain() {
        if rescan_and_merge(service, &mut status, &repo) {
            any_changed = true;
            if let Some(entry) = status.repos.iter().find(|r| r.path == repo).cloned() {
                ambient_upgrades.set(repo, entry);
            }
        }
    }
    if any_changed {
        if let Err(e) = status_writer.write(&status) {
            error!(error = %e, "Failed to write status file after rescan");
        } else {
            debug!(repos = status.repos.len(), "Incremental status written");
        }
    }
}

/// Idempotently repaint the Git-Same folder icon on every workspace root in
/// `status`. Skips roots whose `Icon\r` file is already present (the normal
/// case) so the hot path is one stat per workspace. Recovers gracefully if
/// the user manually deleted the icon. Opt-out via
/// `[ui] custom_folder_icon = false`.
fn reapply_workspace_folder_icons(config: &Config, status: &FinderStatus) {
    if !config.ui.custom_folder_icon {
        return;
    }
    for ws in &status.workspaces {
        if crate::macos::folder_icon::is_set(&ws.root) {
            continue;
        }
        crate::macos::folder_icon::set_or_log(
            &ws.root,
            crate::macos::folder_icon::WORKSPACE_FOLDER_ICNS,
        );
    }
}

fn collect_watched_roots(config: &Config, status: &FinderStatus) -> Vec<PathBuf> {
    let mut roots: Vec<PathBuf> = Vec::new();
    for ws in &status.workspaces {
        let canonical = std::fs::canonicalize(&ws.root).unwrap_or_else(|_| ws.root.clone());
        if !roots.contains(&canonical) {
            roots.push(canonical);
        }
    }
    if config.finder.show_ambient {
        for raw in &config.finder.scan_roots {
            let expanded = shellexpand::tilde(raw).to_string();
            let path = PathBuf::from(expanded);
            if !path.exists() {
                continue;
            }
            let canonical = std::fs::canonicalize(&path).unwrap_or(path);
            if !roots.contains(&canonical) {
                roots.push(canonical);
            }
        }
    }
    roots
}

fn start_filesystem_watcher(
    roots: &[PathBuf],
    tx: tokio::sync::mpsc::UnboundedSender<PathBuf>,
) -> Result<RecommendedWatcher> {
    let watch_roots: Vec<PathBuf> = roots.to_vec();
    let mut watcher = RecommendedWatcher::new(
        move |res: notify::Result<notify::Event>| {
            let event = match res {
                Ok(event) => event,
                Err(e) => {
                    debug!(error = %e, "notify error");
                    return;
                }
            };
            for raw_path in event.paths {
                let canonical =
                    std::fs::canonicalize(&raw_path).unwrap_or_else(|_| raw_path.clone());
                if let Some(repo) = enclosing_repo(&canonical, &watch_roots) {
                    let _ = tx.send(repo);
                }
            }
        },
        notify::Config::default(),
    )
    .map_err(|e| crate::errors::AppError::config(format!("notify watcher init failed: {e}")))?;
    for root in roots {
        if let Err(e) = watcher.watch(root, RecursiveMode::Recursive) {
            warn!(path = %root.display(), error = %e, "Failed to watch root");
        }
    }
    Ok(watcher)
}

/// Walk up from `path` until a `.git` directory is found, stopping at the
/// parent of any watched root. Returns the repo's working-tree path.
fn enclosing_repo(path: &Path, watched_roots: &[PathBuf]) -> Option<PathBuf> {
    let mut current = path;
    loop {
        if current.join(".git").exists() {
            return Some(current.to_path_buf());
        }
        if watched_roots.iter().any(|r| r == current) {
            return None;
        }
        current = current.parent()?;
    }
}
