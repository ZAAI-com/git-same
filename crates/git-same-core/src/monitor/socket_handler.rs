//! Handles a single connection on the monitor's Unix socket.
//!
//! Each accepted connection is text-line based: read one line, dispatch
//! the corresponding `DaemonCommand`, write a one-line response.

use crate::api::{AmbientUpgradeCache, OwnerTypeCache, RepoScanService};
use crate::config::Config;
use crate::git::ShellGit;
use crate::ipc::unix_socket::DaemonCommand;
use crate::ipc::StatusFileWriter;
use std::path::Path;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tracing::{debug, error};

/// Read one command from `stream`, run it against the live state, write
/// the response, and close. Errors are logged and swallowed; a misbehaving
/// client must not take the monitor down.
pub async fn handle_socket_connection(
    mut stream: UnixStream,
    config: &Config,
    pid: u32,
    status_path: &Path,
    owner_types: Option<OwnerTypeCache>,
    ambient_upgrades: Option<AmbientUpgradeCache>,
) {
    let (reader, mut writer) = stream.split();
    let mut reader = BufReader::new(reader);
    let mut line = String::new();

    match reader.read_line(&mut line).await {
        Ok(0) => return,
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
