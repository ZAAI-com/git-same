//! Monitor loop entry point.

use crate::api::{AmbientUpgradeCache, OwnerTypeCache, RepoScanService};
use crate::config::Config;
use crate::errors::Result;
use crate::git::ShellGit;
use crate::ipc::status_file::ensure_legacy_symlinks;
use crate::ipc::{IpcConfig, StatusFileWriter};
use crate::output::Output;
use std::future::Future;
use std::time::Duration;
use tracing::{debug, error, info, warn};

use super::owner_classifier::spawn_owner_classifier;

/// Options for [`run`].
#[derive(Debug, Clone)]
pub struct Options {
    /// How long to sleep between background scans.
    pub interval: Duration,
    /// Resolved IPC paths (status file + socket).
    pub ipc_config: IpcConfig,
}

/// Run the monitor loop until `shutdown` resolves.
///
/// The caller owns the shutdown future so each host can wire whichever
/// termination signal makes sense for it. The CLI composes
/// `tokio::signal::ctrl_c()` with SIGTERM; an embedded host can use a
/// `tokio::sync::Notify` to stop the loop on app exit.
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

    let status_writer = StatusFileWriter::new(ipc_config.status_file_path());
    let git = ShellGit::new();

    let owner_types = OwnerTypeCache::load(OwnerTypeCache::default_path(&ipc_config.dir));
    let ambient_upgrades = AmbientUpgradeCache::new();
    let service = RepoScanService::new(&git, config)
        .with_owner_types(owner_types.clone())
        .with_ambient_upgrades(ambient_upgrades.clone());
    spawn_owner_classifier(config.clone(), owner_types);

    let pid = std::process::id();

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

    #[cfg(unix)]
    let socket_listener = crate::ipc::UnixSocketListener::new(ipc_config.socket_path());
    #[cfg(unix)]
    let tokio_listener = socket_listener.bind().await?;

    tokio::pin!(shutdown);

    loop {
        #[cfg(unix)]
        {
            tokio::select! {
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
                result = tokio_listener.accept() => {
                    match result {
                        Ok((stream, _)) => {
                            let config_clone = config.clone();
                            let writer_path = status_writer.path().to_path_buf();
                            let owner_clone = service.owner_types_clone();
                            let ambient_clone = service.ambient_upgrades_clone();
                            tokio::spawn(async move {
                                super::socket_handler::handle_socket_connection(
                                    stream,
                                    &config_clone,
                                    pid,
                                    &writer_path,
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
