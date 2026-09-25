//! Monitor loop entry point.
//!
//! Event-driven scans (notify FSEvents + socket REFRESH) are the primary
//! source of updates; a periodic full `scan_all` is kept as a safety net
//! for events `notify` may have dropped and for ambient repos that appear
//! in scan roots without a parent we are subscribed to. The full-scan
//! cadence is controlled by `Options::interval` (in turn driven by the CLI
//! `--interval` flag and `config.monitor.fullscan_interval_secs`), stretched
//! when a pass is slow so full scans never run back to back.
//!
//! The loop runs every scan and is the only writer of `status.json`. Socket
//! tasks answer at once and hand refresh requests to the loop as
//! [`ScanRequest`]s.
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
use std::ffi::OsStr;
use std::future::Future;
use std::path::{Component, Path, PathBuf};
use std::time::Duration;
use tokio::time::Instant;
use tracing::{debug, error, info, warn};

use super::owner_classifier::spawn_owner_classifier;

/// Trailing-edge debounce window for collapsing FSEvent bursts (e.g. a
/// single `git commit` fires many .git/ writes) into one repo rescan.
const FS_EVENT_DEBOUNCE: Duration = Duration::from_millis(750);

/// Lower bound for the full-scan cadence so a zero in config cannot spin.
const MIN_FULLSCAN_INTERVAL: Duration = Duration::from_secs(5);

/// A full scan is followed by at least this many times its own duration
/// before the next one starts, so even a slow pass leaves the loop idle about
/// four fifths of the time.
const FULL_SCAN_REST_FACTOR: u32 = 4;

/// Stand-in for "never" when a deadline would overflow `Instant` (the same
/// 30-year horizon tokio uses).
const FAR_FUTURE: Duration = Duration::from_secs(86_400 * 365 * 30);

/// Work a socket client asked the monitor loop to do.
///
/// Socket tasks only queue requests: the loop runs every scan and is the
/// only writer of `status.json`, so a client is answered without waiting for
/// a scan and two scans never race on the status file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScanRequest {
    /// `REFRESH_ALL`: run the safety-net full scan now. Any number of
    /// requests that arrive before it starts collapse into that one scan.
    Full,
    /// `REFRESH <path>`: rescan this canonical path with the next flush.
    Repo(PathBuf),
}

/// Options for [`run`].
#[derive(Debug, Clone)]
pub struct Options {
    /// Cadence of the safety-net full `scan_all`. Most updates flow through
    /// the FSEvents arm; this timer covers dropped events and catches
    /// ambient repos that appear without a parent we subscribed to. The
    /// loop waits longer after a slow pass (see `full_scan_delay`).
    pub interval: Duration,
    /// Resolved IPC paths (status file + socket).
    pub ipc_config: IpcConfig,
}

impl Options {
    /// Build options from `config.toml`. An explicit `interval_override` (the
    /// CLI `--interval` flag) wins over `[monitor] fullscan_interval_secs`.
    pub fn from_config(
        config: &Config,
        ipc_config: IpcConfig,
        interval_override: Option<u64>,
    ) -> Self {
        let secs = interval_override.unwrap_or(config.monitor.fullscan_interval_secs);
        Self {
            interval: Duration::from_secs(secs),
            ipc_config,
        }
    }
}

/// Resolve when the process receives SIGINT (ctrl-c) or SIGTERM (`gisa
/// monitor --stop`, `launchctl bootout`). Shared by every monitor host: the
/// CLI subcommand and the app's headless monitor mode.
pub async fn default_shutdown_signal() {
    #[cfg(unix)]
    {
        let mut sigterm =
            match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
                Ok(signal) => signal,
                Err(_) => {
                    let _ = tokio::signal::ctrl_c().await;
                    return;
                }
            };
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {},
            _ = sigterm.recv() => {},
        }
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
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
            let error = MonitorAgentError::io("Failed to acquire the monitor runtime lock", e);
            if managed {
                error!(error = %error, "Cannot use the runtime lock; not starting");
                return Ok(());
            }
            return Err(error.into());
        }
    };

    if let Err(e) = ensure_legacy_symlinks(&ipc_config.dir) {
        warn!("Could not refresh legacy IPC symlinks: {}", e);
    }

    info!("Starting git-same monitor");
    output.info("Starting git-same monitor...");

    let live = LiveConfig::new(config.clone(), context.config_path.clone());
    let status_writer = ipc_config.status_writer();
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

    let scan_started = Instant::now();
    let initial_result = scan(&live.snapshot());
    let mut last_scan = scan_started.elapsed();
    let initial_status = match initial_result {
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
        took = ?last_scan,
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
    // Owned by the loop alone: socket tasks never read or write it.
    let mut status = initial_status;

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
    // Socket tasks queue refresh requests here; the loop runs the scans.
    let (scan_tx, mut scan_rx) = tokio::sync::mpsc::unbounded_channel::<ScanRequest>();

    let mut pending: HashSet<PathBuf> = HashSet::new();
    // The subset of `pending` a client named in `REFRESH <path>`.
    let mut explicit: HashSet<PathBuf> = HashSet::new();
    let mut interval = interval.max(MIN_FULLSCAN_INTERVAL);
    // A deadline, not a per-iteration sleep: a steady stream of filesystem
    // events must not postpone the safety-net scan forever.
    let mut next_full_scan = deadline_after(Instant::now(), full_scan_delay(interval, last_scan));

    tokio::pin!(shutdown);

    loop {
        let debounce_active = !pending.is_empty();
        let mut config_changed = false;
        let mut rescanned = false;

        tokio::select! {
            // Polled in this order. Shutdown and accept only become ready on a
            // real signal or connection, so they cannot starve the rest, and a
            // client is answered before a due scan occupies the loop.
            biased;
            _ = &mut shutdown => {
                info!("Monitor shutting down");
                output.info("Monitor shutting down...");
                #[cfg(unix)]
                socket_listener.cleanup();
                break;
            },
            connection = next_connection(&tokio_listener) => {
                serve_connection(
                    connection,
                    ConnectionState {
                        live: live.clone(),
                        reload_tx: reload_tx.clone(),
                        scan_tx: scan_tx.clone(),
                        status_writer: status_writer.clone(),
                    },
                );
            },
            Some(request) = scan_rx.recv() => {
                apply_scan_request(
                    request,
                    Instant::now(),
                    &mut next_full_scan,
                    &mut pending,
                    &mut explicit,
                );
            },
            Some(()) = reload_rx.recv() => {
                config_changed = true;
            },
            _ = tokio::time::sleep_until(next_full_scan) => {
                debug!("Safety-net full scan");
                config_changed = live.reload_if_changed();
                let config = live.snapshot();
                let started = Instant::now();
                match scan(&config) {
                    Ok(new_status) => {
                        reapply_workspace_folder_icons(&config, &new_status);
                        status = new_status;
                        rescanned = true;
                        if let Err(e) = status_writer.write(&status) {
                            error!(error = %e, "Failed to write status file after full scan");
                        } else {
                            debug!(
                                repos = status.repos.len(),
                                took = ?started.elapsed(),
                                "Full scan complete"
                            );
                        }
                    }
                    Err(e) => {
                        error!(error = %e, "Full scan failed");
                    }
                }
                last_scan = started.elapsed();
                next_full_scan =
                    deadline_after(Instant::now(), full_scan_delay(interval, last_scan));
            },
            Some(repo_path) = fs_rx.recv() => {
                pending.insert(repo_path);
            },
            _ = tokio::time::sleep(FS_EVENT_DEBOUNCE), if debounce_active => {
                let config = live.snapshot();
                let service = RepoScanService::new(&git, &config)
                    .with_owner_types(owner_types.clone())
                    .with_ambient_upgrades(ambient_upgrades.clone());
                flush_pending(
                    &service,
                    &mut status,
                    &status_writer,
                    &ambient_upgrades,
                    &mut pending,
                    &mut explicit,
                );
            },
        }

        // A reload can change the configured roots, and a full scan can find
        // workspace roots that appeared or vanished (including one that a
        // `REFRESH_ALL` reload registered before its scan ran).
        if config_changed || rescanned {
            let roots = collect_watched_roots(&live.snapshot(), &status);
            if roots != watched_roots {
                info!(
                    roots = roots.len(),
                    "Watched roots changed; restarting the filesystem watcher"
                );
                watched_roots = roots;
                watcher = start_watcher_or_warn(&watched_roots, fs_tx.clone());
            }
        }
        if config_changed {
            let config = live.snapshot();
            if !context.interval_explicit {
                let configured = Duration::from_secs(config.monitor.fullscan_interval_secs)
                    .max(MIN_FULLSCAN_INTERVAL);
                if configured != interval {
                    interval = configured;
                    next_full_scan =
                        deadline_after(Instant::now(), full_scan_delay(interval, last_scan));
                }
            }
            // Cheap when nothing is missing: the classifier returns early.
            spawn_owner_classifier((*config).clone(), owner_types.clone());
        }
    }

    drop(watcher);
    Ok(())
}

/// Delay from the end of one full scan to the start of the next: the
/// configured interval, stretched to `FULL_SCAN_REST_FACTOR` times the last
/// pass when that pass was slow, so full scans never run back to back.
fn full_scan_delay(interval: Duration, last_scan: Duration) -> Duration {
    interval
        .max(MIN_FULLSCAN_INTERVAL)
        .max(last_scan.saturating_mul(FULL_SCAN_REST_FACTOR))
}

/// `now + delay`, saturating instead of panicking on an absurd configured
/// interval.
fn deadline_after(now: Instant, delay: Duration) -> Instant {
    now.checked_add(delay).unwrap_or_else(|| now + FAR_FUTURE)
}

/// Fold one socket request into the loop's schedule.
///
/// `Full` only pulls the deadline forward, so any number of requests that
/// arrive before the scan starts collapse into it. `Repo` joins the
/// debounced flush and is marked explicit (see [`flush_pending`]).
fn apply_scan_request(
    request: ScanRequest,
    now: Instant,
    next_full_scan: &mut Instant,
    pending: &mut HashSet<PathBuf>,
    explicit: &mut HashSet<PathBuf>,
) {
    match request {
        ScanRequest::Full => *next_full_scan = (*next_full_scan).min(now),
        ScanRequest::Repo(path) => {
            pending.insert(path.clone());
            explicit.insert(path);
        }
    }
}

/// Everything a socket task needs, cloned out of the loop.
// Only the `#[cfg(unix)]` `serve_connection` reads these fields; off Unix the
// struct is still built but never consumed, so every field reads as dead.
#[cfg_attr(not(unix), allow(dead_code))]
struct ConnectionState {
    live: LiveConfig,
    reload_tx: tokio::sync::mpsc::UnboundedSender<()>,
    scan_tx: tokio::sync::mpsc::UnboundedSender<ScanRequest>,
    status_writer: StatusFileWriter,
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
            &state.scan_tx,
            &state.status_writer,
            super::socket_handler::CLIENT_TIMEOUT,
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

/// Rescan every pending repo and write the status file once if anything
/// changed.
///
/// A changed repo refreshes its ambient-upgrade entry. So does every repo in
/// `explicit` (named by a client in `REFRESH <path>`) even when unchanged:
/// that request is how a right-click upgrades a gray ambient repo, and the
/// entry is what keeps the upgrade across later full scans.
fn flush_pending(
    service: &RepoScanService<'_>,
    status: &mut FinderStatus,
    status_writer: &StatusFileWriter,
    ambient_upgrades: &AmbientUpgradeCache,
    pending: &mut HashSet<PathBuf>,
    explicit: &mut HashSet<PathBuf>,
) {
    if pending.is_empty() {
        return;
    }
    let mut any_changed = false;
    for repo in pending.drain() {
        let changed = rescan_and_merge(service, status, &repo);
        any_changed |= changed;
        if changed || explicit.contains(&repo) {
            if let Some(entry) = status.repos.iter().find(|r| r.path == repo).cloned() {
                ambient_upgrades.set(repo, entry);
            }
        }
    }
    explicit.clear();
    if any_changed {
        if let Err(e) = status_writer.write(status) {
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
                if !is_badge_relevant(&raw_path) {
                    continue;
                }
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

/// Whether a raw watcher path can change a repo's badge.
///
/// Working-tree paths always can, lock files included (`Cargo.lock` is
/// content). Inside `.git` only branch, index, config, ref and
/// in-progress-operation state can; objects, logs, `FETCH_HEAD` and other
/// tools' scratch files cannot. Forwarding those would make every scan the
/// trigger for the next one, since any git command (the monitor's own
/// `git status` included) may touch them. Lock files inside `.git` never
/// pass: git writes `x.lock` and renames it over `x`, so the event for `x`
/// still arrives.
fn is_badge_relevant(path: &Path) -> bool {
    let dot_git = Component::Normal(OsStr::new(".git"));
    let mut components = path.components();
    if !components.by_ref().any(|component| component == dot_git) {
        return true;
    }
    // Names inside `.git` are ASCII; anything else is none of git's files.
    let Some(inside) = components
        .map(|component| component.as_os_str().to_str())
        .collect::<Option<Vec<&str>>>()
    else {
        return false;
    };
    if inside.last().is_some_and(|name| name.ends_with(".lock")) {
        return false;
    }
    matches!(
        inside.as_slice(),
        ["HEAD"
            | "index"
            | "config"
            | "packed-refs"
            | "MERGE_HEAD"
            | "CHERRY_PICK_HEAD"
            | "REVERT_HEAD"]
            | ["refs", ..]
            | ["logs", "refs", "stash"]
            | ["rebase-merge" | "rebase-apply", ..]
            | ["worktrees", _, "HEAD" | "index"]
    )
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
