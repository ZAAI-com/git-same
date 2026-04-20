//! Daemon command handler.
//!
//! Runs a background daemon that monitors workspace repositories,
//! computes Finder badge status, and writes the status JSON file.
//! Listens on a Unix socket for refresh requests from the Finder extension.
//!
//! All scanning logic lives in `crate::api::RepoScanService`. This module
//! is just the CLI surface (start/stop/status) plus the daemon loop and
//! socket handler that drive the service.

use crate::api::{AmbientUpgradeCache, OwnerTypeCache, RepoScanService};
use crate::cli::DaemonArgs;
use crate::config::Config;
use crate::errors::Result;
use crate::git::ShellGit;
use crate::ipc::{IpcConfig, StatusFileWriter};
use crate::output::Output;
use crate::types::OwnerType;
use std::path::Path;
use tracing::{debug, error, info, warn};

/// Run the daemon command.
pub async fn run(args: &DaemonArgs, config: &Config, output: &Output) -> Result<()> {
    let ipc_config = IpcConfig::default_path()?;
    ipc_config.ensure_dir()?;

    // Handle --status: check if daemon is running
    if args.status {
        return show_status(&ipc_config, output);
    }

    // Handle --stop: terminate running daemon
    if args.stop {
        return stop_daemon(&ipc_config, output);
    }

    // Start the daemon
    info!("Starting git-same daemon");
    output.info("Starting git-same daemon...");

    let status_writer = StatusFileWriter::new(ipc_config.status_file_path());
    let git = ShellGit::new();

    let owner_types = OwnerTypeCache::load(OwnerTypeCache::default_path(&ipc_config.dir));
    let ambient_upgrades = AmbientUpgradeCache::new();
    let service = RepoScanService::new(&git, config)
        .with_owner_types(owner_types.clone())
        .with_ambient_upgrades(ambient_upgrades.clone());
    spawn_owner_classifier(config.clone(), owner_types);

    let pid = std::process::id();

    // Initial scan
    let finder_status = service.scan_all(pid)?;
    status_writer.write(&finder_status)?;
    let ambient_count = finder_status
        .repos
        .iter()
        .filter(|r| r.workspace.is_none())
        .count();
    let workspace_count = finder_status.repos.len() - ambient_count;
    info!(
        repos = finder_status.repos.len(),
        workspace = workspace_count,
        ambient = ambient_count,
        "Initial scan complete, status written"
    );
    output.info(&format!(
        "Monitoring {} repos ({} workspace, {} ambient). Status: {}",
        finder_status.repos.len(),
        workspace_count,
        ambient_count,
        ipc_config.status_file_path().display()
    ));

    // Set up Unix socket listener
    #[cfg(unix)]
    let socket_listener = crate::ipc::UnixSocketListener::new(ipc_config.socket_path());

    #[cfg(unix)]
    let tokio_listener = socket_listener.bind().await?;

    // Main daemon loop
    let interval = tokio::time::Duration::from_secs(args.interval);

    // Listen for SIGTERM in addition to SIGINT so `gisa daemon --stop`
    // (which sends SIGTERM) triggers clean socket cleanup.
    #[cfg(unix)]
    let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;

    loop {
        tokio::select! {
            // Wait for the polling interval
            _ = tokio::time::sleep(interval) => {
                debug!("Polling interval reached, scanning...");
                match service.scan_all(pid) {
                    Ok(status) => {
                        if let Err(e) = status_writer.write(&status) {
                            error!(error = %e, "Failed to write status file");
                        } else {
                            debug!(repos = status.repos.len(), "Scan complete");
                        }
                    }
                    Err(e) => {
                        error!(error = %e, "Scan failed");
                    }
                }
            },
            // Accept socket connections for refresh requests
            result = tokio_listener.accept() => {
                match result {
                    Ok((stream, _)) => {
                        let config_clone = config.clone();
                        let writer_path = status_writer.path().to_path_buf();
                        let owner_clone = service.owner_types_clone();
                        let ambient_clone = service.ambient_upgrades_clone();
                        tokio::spawn(async move {
                            handle_socket_connection(stream, &config_clone, pid, &writer_path, owner_clone, ambient_clone).await;
                        });
                    }
                    Err(e) => {
                        warn!(error = %e, "Failed to accept socket connection");
                    }
                }
            },
            // Handle shutdown signals (SIGINT from ctrl-c, SIGTERM from `daemon --stop`)
            _ = tokio::signal::ctrl_c() => {
                info!("Received SIGINT");
                output.info("Daemon shutting down...");
                socket_listener.cleanup();
                break;
            },
            _ = sigterm.recv() => {
                info!("Received SIGTERM");
                output.info("Daemon shutting down...");
                socket_listener.cleanup();
                break;
            },
        }
    }

    Ok(())
}

/// Handle a single socket connection.
#[cfg(unix)]
async fn handle_socket_connection(
    mut stream: tokio::net::UnixStream,
    config: &Config,
    pid: u32,
    status_path: &Path,
    owner_types: Option<OwnerTypeCache>,
    ambient_upgrades: Option<AmbientUpgradeCache>,
) {
    use crate::ipc::unix_socket::DaemonCommand;
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

    let (reader, mut writer) = stream.split();
    let mut reader = BufReader::new(reader);
    let mut line = String::new();

    match reader.read_line(&mut line).await {
        Ok(0) => return, // connection closed
        Ok(_) => {}
        Err(e) => {
            debug!(error = %e, "Failed to read from socket");
            return;
        }
    }

    let cmd = DaemonCommand::parse(&line);
    let git = ShellGit::new();
    let mut service = RepoScanService::new(&git, config);
    if let Some(cache) = owner_types {
        service = service.with_owner_types(cache);
    }
    if let Some(cache) = ambient_upgrades.clone() {
        service = service.with_ambient_upgrades(cache);
    }

    let response = match cmd {
        DaemonCommand::Ping => "PONG\n".to_string(),
        DaemonCommand::RefreshAll | DaemonCommand::Refresh(_) => {
            // If the client asked to refresh a specific path, run the full
            // scan for it first and store the upgraded entry in the ambient
            // cache. The subsequent `scan_all` will pick it up automatically.
            if let DaemonCommand::Refresh(ref path) = cmd {
                let canonical = std::fs::canonicalize(path).unwrap_or_else(|_| path.clone());
                debug!(path = %canonical.display(), "Refresh requested");
                let upgraded = service.scan_repo(&canonical, None, None);
                if let Some(cache) = &ambient_upgrades {
                    cache.set(canonical, upgraded);
                }
            }
            match service.scan_all(pid) {
                Ok(status) => {
                    let file_writer = StatusFileWriter::new(status_path.to_path_buf());
                    let _ = file_writer.write(&status);
                }
                Err(e) => error!(error = %e, "Refresh failed"),
            }
            "OK\n".to_string()
        }
        DaemonCommand::Status => {
            let file_writer = StatusFileWriter::new(status_path.to_path_buf());
            match file_writer.read() {
                Ok(status) => {
                    serde_json::to_string_pretty(&status).unwrap_or_else(|_| "ERROR\n".to_string())
                }
                Err(_) => "ERROR\n".to_string(),
            }
        }
        DaemonCommand::Unknown(cmd) => {
            format!("UNKNOWN: {}\n", cmd)
        }
    };

    let _ = writer.write_all(response.as_bytes()).await;
    let _ = writer.flush().await;
}

/// Show daemon status.
///
/// User-facing diagnostic output is printed directly so it is not suppressed
/// by the default Quiet verbosity: `--status` must always answer.
fn show_status(ipc_config: &IpcConfig, _output: &Output) -> Result<()> {
    let status_path = ipc_config.status_file_path();
    if !status_path.exists() {
        println!("Daemon is not running (no status file found)");
        return Ok(());
    }

    let writer = StatusFileWriter::new(status_path);
    match writer.read() {
        Ok(status) => {
            let pid = status.daemon_pid;
            if is_process_alive(pid) {
                println!("Daemon is running (PID: {})", pid);
            } else {
                println!("Daemon is not running (stale PID: {})", pid);
            }
            println!("Last scan: {}", status.timestamp);
            println!("Repos monitored: {}", status.repos.len());
            println!(
                "Workspaces: {}",
                status
                    .workspaces
                    .iter()
                    .map(|w| w.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
        Err(e) => {
            eprintln!("Could not read status file: {}", e);
        }
    }
    Ok(())
}

/// Stop a running daemon.
fn stop_daemon(ipc_config: &IpcConfig, _output: &Output) -> Result<()> {
    let status_path = ipc_config.status_file_path();
    if !status_path.exists() {
        println!("No daemon is running");
        return Ok(());
    }

    let writer = StatusFileWriter::new(status_path);
    match writer.read() {
        Ok(status) => {
            let pid = status.daemon_pid;
            if is_process_alive(pid) {
                let _ = std::process::Command::new("kill")
                    .args(["-TERM", &pid.to_string()])
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .status();
                println!("Sent stop signal to daemon (PID: {})", pid);
            } else {
                println!("Daemon is not running (stale status file)");
            }
        }
        Err(_) => {
            println!("Could not read daemon status");
        }
    }
    Ok(())
}

/// Spawn a background task that classifies every org folder name in the
/// workspace config as User or Organization via the GitHub API and persists
/// the result in `OwnerTypeCache`. Subsequent periodic scans pick up the new
/// classifications as the cache fills.
fn spawn_owner_classifier(config: Config, cache: OwnerTypeCache) {
    tokio::spawn(async move {
        let names = collect_owner_names(&config);
        let missing = cache.missing(names.iter().map(|s| s.as_str()));
        if missing.is_empty() {
            debug!("Owner type cache already populated, skipping classification");
            return;
        }

        let token = match crate::auth::gh_cli::get_token() {
            Ok(t) => t,
            Err(e) => {
                warn!(error = %e, "Owner classification skipped: gh auth token unavailable");
                return;
            }
        };
        let ws_provider = crate::config::WorkspaceProvider::default();
        let provider = match crate::provider::create_provider(&ws_provider, &token) {
            Ok(p) => p,
            Err(e) => {
                warn!(error = %e, "Owner classification skipped: provider init failed");
                return;
            }
        };

        info!(
            count = missing.len(),
            "Classifying owner types via GitHub API"
        );
        for name in &missing {
            match provider.get_owner_type(name).await {
                Ok(ot) => {
                    if let Err(e) = cache.set(name, ot) {
                        warn!(name = %name, error = %e, "Failed to persist owner type");
                    } else {
                        debug!(name = %name, owner_type = ?ot, "Classified owner");
                    }
                }
                Err(e) => {
                    debug!(name = %name, error = %e, "Owner classification failed, leaving unknown");
                    // Cache a "last tried" marker to avoid retrying every scan
                    let _ = cache.set(name, OwnerType::Unknown);
                }
            }
        }
        info!("Owner classification complete");
    });
}

/// Collect all unique top-level folder names (orgs + users) from every
/// configured workspace. Mirrors the scanning logic in `RepoScanService`.
fn collect_owner_names(config: &Config) -> Vec<String> {
    use std::collections::BTreeSet;
    let mut names: BTreeSet<String> = BTreeSet::new();

    for ws_path in &config.workspaces {
        let expanded = shellexpand::tilde(ws_path).to_string();
        let root = std::path::PathBuf::from(&expanded);
        if !root.exists() {
            continue;
        }
        let ws_config = match crate::config::WorkspaceStore::load(&root) {
            Ok(ws) => ws,
            Err(_) => continue,
        };
        let base_path = ws_config.expanded_base_path();
        if !ws_config.orgs.is_empty() {
            names.extend(ws_config.orgs.iter().cloned());
        } else if let Ok(entries) = std::fs::read_dir(&base_path) {
            for e in entries.flatten() {
                if let Some(n) = e.file_name().to_str() {
                    if !n.starts_with('.') && e.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                        names.insert(n.to_string());
                    }
                }
            }
        }
        if !ws_config.username.is_empty() {
            names.insert(ws_config.username.clone());
        }
    }

    names.into_iter().collect()
}

/// Check if a process with the given PID is alive.
fn is_process_alive(pid: u32) -> bool {
    // Use `kill -0` via shell — avoids libc dependency
    std::process::Command::new("kill")
        .args(["-0", &pid.to_string()])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

#[cfg(test)]
#[path = "daemon_tests.rs"]
mod tests;
