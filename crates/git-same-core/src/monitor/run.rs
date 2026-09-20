//! Monitor loop entry point.
//!
//! Event-driven scans (notify FSEvents + socket REFRESH) are the primary
//! source of updates; a periodic full `scan_all` is kept as a safety net
//! for events `notify` may have dropped and for ambient repos that appear
//! in scan roots without a parent we are subscribed to. The full-scan
//! cadence is controlled by `Options::interval` (in turn driven by the CLI
//! `--interval` flag and `config.monitor.fullscan_interval_secs`).
//!
//! Startup order matters: the runtime lock is taken before anything is
//! written or any existing socket is removed, so a second monitor can never
//! steal the socket or race the status file. The socket is bound only after
//! the initial scan, so "lock held, socket absent" means "starting".

use crate::api::{AmbientUpgradeCache, OwnerTypeCache, RepoScanService};
use crate::config::Config;
use crate::errors::{MonitorAgentError, Result};
use crate::git::ShellGit;
use crate::ipc::status_file::ensure_legacy_symlinks;
use crate::ipc::{IpcConfig, StatusFileWriter};
use crate::monitor::incremental::rescan_and_merge;
use crate::monitor::live_config::LiveConfig;
use crate::monitor::runtime_guard::{AcquireError, MonitorMode, RuntimeGuard};
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

/// Lower bound for the full-scan cadence so a zero in config cannot spin.
const MIN_FULLSCAN_INTERVAL: Duration = Duration::from_secs(5);

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

/// How and from where this monitor process was started.
///
/// Kept separate from [`Options`] so that struct's public shape stays stable.
#[derive(Debug, Clone)]
pub struct RunContext {
    /// Managed background service or a user-started foreground monitor.
    pub mode: MonitorMode,
    /// File the config was loaded from. When set, the monitor reloads it on
    /// `REFRESH_ALL` and when it changes on disk.
    pub config_path: Option<PathBuf>,
    /// `true` when the cadence came from `--interval`; a reload then never
    /// replaces it with the value from the file.
    pub interval_explicit: bool,
}

/// Run the monitor loop until `shutdown` resolves.
///
/// Foreground monitor with a fixed configuration. See [`run_with`] for the
/// managed service and for configuration reloading.
pub async fn run<S>(config: &Config, output: &Output, opts: Options, shutdown: S) -> Result<()>
where
    S: Future<Output = ()>,
{
    let context = RunContext {
        mode: MonitorMode::Foreground,
        config_path: None,
        interval_explicit: true,
    };
    run_with(config, output, opts, context, shutdown).await
}

/// Run the monitor loop with an explicit [`RunContext`].
pub async fn run_with<S>(
    config: &Config,
    output: &Output,
    opts: Options,
    context: RunContext,
    shutdown: S,
) -> Result<()>
where
    S: Future<Output = ()>,
{
    let Options {
        interval,
        ipc_config,
    } = opts;
    let managed = context.mode == MonitorMode::Managed;

    // Under launchd a nonzero exit means "restart me"
    // (`KeepAlive = { SuccessfulExit = false }`, `ThrottleInterval 10`). Every
    // startup step that a restart cannot fix must therefore exit 0 after one
    // logged line, or the helper respawns every ten seconds forever.
    if let Err(e) = ipc_config.ensure_dir() {
        if managed {
            error!(error = %e, "Cannot use the IPC directory; not starting");
            return Ok(());
        }
        return Err(e);
    }

    // Before writing status or touching the socket: become the only monitor.
    let _guard = match RuntimeGuard::acquire(&ipc_config, context.mode) {
        Ok(guard) => guard,
        Err(AcquireError::Held(holder)) => {
            return Err(MonitorAgentError::AlreadyRunning {
                pid: holder.map(|identity| identity.pid),
            }
            .into());
        }
        Err(AcquireError::Io(e)) => {
            return Err(
                MonitorAgentError::io("Failed to acquire the monitor runtime lock", e).into(),
            );
        }
    };

    if let Err(e) = ensure_legacy_symlinks(&ipc_config.dir) {
        warn!("Could not refresh legacy IPC symlinks: {}", e);
    }

    info!("Starting git-same monitor");
    output.info("Starting git-same monitor...");

    let live = LiveConfig::new(config.clone(), context.config_path.clone());
    let status_writer = StatusFileWriter::new(ipc_config.status_file_path());
    let git = ShellGit::new();

    let owner_types = OwnerTypeCache::load(OwnerTypeCache::default_path(&ipc_config.dir));
    let ambient_upgrades = AmbientUpgradeCache::new();
    spawn_owner_classifier(config.clone(), owner_types.clone());

    let pid = std::process::id();
    let scan = |config: &Config| {
        RepoScanService::new(&git, config)
            .with_owner_types(owner_types.clone())
            .with_ambient_upgrades(ambient_upgrades.clone())
            .scan_all(pid)
    };

    let initial_status = match scan(&live.snapshot()) {
        Ok(status) => status,
        // A persistently failing scan (for example a permission denial) would
        // loop forever under launchd. Stay up with an empty status and let the
        // periodic scan retry.
        Err(e) if managed => {
            error!(error = %e, "Initial scan failed; retrying on the next full scan");
            FinderStatus::new(pid, chrono::Utc::now().to_rfc3339())
        }
        Err(e) => return Err(e),
    };
    if let Err(e) = status_writer.write(&initial_status) {
        // An unwritable group container is not fixed by respawning.
        if managed {
            error!(error = %e, "Cannot write the status file; not starting");
            return Ok(());
        }
        return Err(e);
    }
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

    reapply_workspace_folder_icons(&live.snapshot(), &initial_status);
    let mut watched_roots = collect_watched_roots(&live.snapshot(), &initial_status);
    let shared_status = Arc::new(Mutex::new(initial_status));

    #[cfg(unix)]
    let socket_listener = crate::ipc::UnixSocketListener::new(ipc_config.socket_path());
    #[cfg(unix)]
    let tokio_listener = match socket_listener.bind().await {
        Ok(listener) => Some(listener),
        // A socket owned by another uid, or an unwritable container, cannot be
        // fixed by a restart. Badges read `status.json` and keep working; only
        // `gisa refresh` and the extension's push requests degrade.
        Err(e) if managed => {
            warn!(error = %e, "Could not bind the IPC socket; continuing without it");
            None
        }
        Err(e) => return Err(e),
    };
    #[cfg(not(unix))]
    let tokio_listener = ();

    let (fs_tx, mut fs_rx) = tokio::sync::mpsc::unbounded_channel::<PathBuf>();
    let mut watcher = start_watcher_or_warn(&watched_roots, fs_tx.clone());
    // Socket tasks report a reload here so the loop can rebuild what it owns.
    let (reload_tx, mut reload_rx) = tokio::sync::mpsc::unbounded_channel::<()>();

    let mut pending: HashSet<PathBuf> = HashSet::new();
    let mut interval = interval.max(MIN_FULLSCAN_INTERVAL);
    // A deadline, not a per-iteration sleep: a steady stream of filesystem
    // events must not postpone the safety-net scan forever.
    let mut next_full_scan = tokio::time::Instant::now() + interval;

    tokio::pin!(shutdown);

    loop {
        let debounce_active = !pending.is_empty();
        let mut config_changed = false;

        tokio::select! {
            _ = tokio::time::sleep(FS_EVENT_DEBOUNCE), if debounce_active => {
                let config = live.snapshot();
                let service = RepoScanService::new(&git, &config)
                    .with_owner_types(owner_types.clone())
                    .with_ambient_upgrades(ambient_upgrades.clone());
                flush_pending(&service, &shared_status, &status_writer, &ambient_upgrades, &mut pending);
            },
            _ = tokio::time::sleep_until(next_full_scan) => {
                debug!("Safety-net full scan");
                config_changed = live.reload_if_changed();
                let config = live.snapshot();
                match scan(&config) {
                    Ok(new_status) => {
                        reapply_workspace_folder_icons(&config, &new_status);
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
                next_full_scan = tokio::time::Instant::now() + interval;
            },
            Some(repo_path) = fs_rx.recv() => {
                pending.insert(repo_path);
            },
            Some(()) = reload_rx.recv() => {
                config_changed = true;
            },
            connection = next_connection(&tokio_listener) => {
                serve_connection(
                    connection,
                    ConnectionState {
                        live: live.clone(),
                        reload_tx: reload_tx.clone(),
                        pid,
                        status_path: status_writer.path().to_path_buf(),
                        shared_status: shared_status.clone(),
                        owner_types: owner_types.clone(),
                        ambient_upgrades: ambient_upgrades.clone(),
                    },
                );
            },
            _ = &mut shutdown => {
                info!("Monitor shutting down");
                output.info("Monitor shutting down...");
                #[cfg(unix)]
                socket_listener.cleanup();
                break;
            },
        }

        if config_changed {
            let config = live.snapshot();
            let status = shared_status.lock().expect("status mutex poisoned").clone();
            let roots = collect_watched_roots(&config, &status);
            if roots != watched_roots {
                info!(
                    roots = roots.len(),
                    "Watched roots changed; restarting the filesystem watcher"
                );
                watched_roots = roots;
                watcher = start_watcher_or_warn(&watched_roots, fs_tx.clone());
            }
            if !context.interval_explicit {
                let configured = Duration::from_secs(config.monitor.fullscan_interval_secs)
                    .max(MIN_FULLSCAN_INTERVAL);
                if configured != interval {
                    interval = configured;
                    next_full_scan = tokio::time::Instant::now() + interval;
                }
            }
            // Cheap when nothing is missing: the classifier returns early.
            spawn_owner_classifier((*config).clone(), owner_types.clone());
        }
    }

    drop(watcher);
    Ok(())
}

/// Everything a socket task needs, cloned out of the loop.
// Only the `#[cfg(unix)]` `serve_connection` reads these fields; off Unix the
// struct is still built but never consumed, so every field reads as dead.
#[cfg_attr(not(unix), allow(dead_code))]
struct ConnectionState {
    live: LiveConfig,
    reload_tx: tokio::sync::mpsc::UnboundedSender<()>,
    pid: u32,
    status_path: PathBuf,
    shared_status: Arc<Mutex<FinderStatus>>,
    owner_types: OwnerTypeCache,
    ambient_upgrades: AmbientUpgradeCache,
}

#[cfg(unix)]
type Connection = std::io::Result<tokio::net::UnixStream>;
#[cfg(not(unix))]
type Connection = std::convert::Infallible;

/// Never resolves when the socket could not be bound, so the rest of the loop
/// (scans, status writes, filesystem events) runs unchanged.
#[cfg(unix)]
async fn next_connection(listener: &Option<tokio::net::UnixListener>) -> Connection {
    let Some(listener) = listener else {
        return std::future::pending().await;
    };
    listener.accept().await.map(|(stream, _)| stream)
}

/// No socket on this platform: never resolves.
#[cfg(not(unix))]
async fn next_connection(_listener: &()) -> Connection {
    std::future::pending().await
}

#[cfg(unix)]
fn serve_connection(connection: Connection, state: ConnectionState) {
    let stream = match connection {
        Ok(stream) => stream,
        Err(e) => {
            warn!(error = %e, "Failed to accept socket connection");
            return;
        }
    };
    tokio::spawn(async move {
        super::socket_handler::handle_socket_connection(
            stream,
            &state.live,
            &state.reload_tx,
            state.pid,
            &state.status_path,
            state.shared_status,
            Some(state.owner_types),
            Some(state.ambient_upgrades),
        )
        .await;
    });
}

#[cfg(not(unix))]
fn serve_connection(connection: Connection, _state: ConnectionState) {
    match connection {}
}

fn start_watcher_or_warn(
    roots: &[PathBuf],
    tx: tokio::sync::mpsc::UnboundedSender<PathBuf>,
) -> Option<RecommendedWatcher> {
    match start_filesystem_watcher(roots, tx) {
        Ok(watcher) => Some(watcher),
        Err(e) => {
            warn!(error = %e, "Filesystem watcher failed to start; monitor will only respond to REFRESH commands");
            None
        }
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

#[cfg(test)]
#[path = "run_tests.rs"]
mod tests;
