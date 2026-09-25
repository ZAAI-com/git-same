//! Handles a single connection on the monitor's Unix socket.
//!
//! Each accepted connection is text-line based: read one line, dispatch
//! the corresponding `DaemonCommand`, write a one-line response.
//!
//! Refresh commands never scan here. They queue a [`ScanRequest`] for the
//! monitor loop and are answered "OK" at once, so a client never waits for a
//! scan; the loop runs every scan and is the only writer of `status.json`.

use crate::ipc::unix_socket::DaemonCommand;
use crate::ipc::StatusFileWriter;
use crate::monitor::live_config::LiveConfig;
use crate::monitor::run::ScanRequest;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::sync::mpsc::UnboundedSender;
use tracing::debug;

/// How long a client may take to send its command line, and again to take
/// the response, before the connection is dropped. Keeps an idle or stuck
/// client from pinning a task forever.
pub const CLIENT_TIMEOUT: Duration = Duration::from_secs(5);

/// Read one command from `stream`, answer it, and close. Errors are logged
/// and swallowed; a misbehaving client must not take the monitor down.
///
/// `REFRESH_ALL` first reloads the configuration if it changed on disk, so a
/// nudge after registering a workspace reaches a monitor that stays running.
/// "OK" to `REFRESH` and `REFRESH_ALL` means the scan is queued; the loop
/// runs it and rewrites `status.json` afterwards.
pub async fn handle_socket_connection(
    mut stream: UnixStream,
    live: &LiveConfig,
    reload_tx: &UnboundedSender<()>,
    scan_tx: &UnboundedSender<ScanRequest>,
    status_writer: &StatusFileWriter,
    timeout: Duration,
) {
    let (reader, mut writer) = stream.split();
    let mut reader = BufReader::new(reader);
    let mut line = String::new();

    match tokio::time::timeout(timeout, reader.read_line(&mut line)).await {
        Err(_) => {
            debug!("Socket client sent no command in time; dropping it");
            return;
        }
        Ok(Ok(0)) => return,
        Ok(Ok(_)) => {}
        Ok(Err(e)) => {
            debug!(error = %e, "Failed to read from socket");
            return;
        }
    }

    let response = match DaemonCommand::parse(&line) {
        DaemonCommand::Ping => "PONG\n".to_string(),
        DaemonCommand::Refresh(path) => {
            let canonical = std::fs::canonicalize(&path).unwrap_or(path);
            debug!(path = %canonical.display(), "Refresh requested");
            queue(scan_tx, ScanRequest::Repo(canonical))
        }
        DaemonCommand::RefreshAll => {
            if live.reload_if_changed() {
                let _ = reload_tx.send(());
            }
            debug!("Full refresh requested");
            queue(scan_tx, ScanRequest::Full)
        }
        DaemonCommand::Status => status_response(status_writer),
        DaemonCommand::Unknown(cmd) => {
            format!("UNKNOWN: {}\n", cmd)
        }
    };

    let reply = async {
        writer.write_all(response.as_bytes()).await?;
        writer.flush().await
    };
    if tokio::time::timeout(timeout, reply).await.is_err() {
        debug!("Socket client did not take the response in time; dropping it");
    }
}

/// Hand `request` to the monitor loop. Only fails once the loop has exited,
/// when nothing is left to run the scan.
fn queue(scan_tx: &UnboundedSender<ScanRequest>, request: ScanRequest) -> String {
    match scan_tx.send(request) {
        Ok(()) => "OK\n".to_string(),
        Err(_) => "ERROR\n".to_string(),
    }
}

/// Build the response for a `STATUS` command: the current status file as
/// pretty JSON terminated by a newline so it matches the line-framed protocol
/// (`PONG\n`, `OK\n`, `ERROR\n`). Returns `ERROR\n` if the file can't be read
/// or serialized.
fn status_response(writer: &StatusFileWriter) -> String {
    match writer.read() {
        Ok(status) => serde_json::to_string_pretty(&status)
            .map(|s| format!("{s}\n"))
            .unwrap_or_else(|_| "ERROR\n".to_string()),
        Err(_) => "ERROR\n".to_string(),
    }
}

#[cfg(test)]
#[path = "socket_handler_tests.rs"]
mod tests;
