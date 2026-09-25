//! Unix domain socket IPC for macOS and Linux.
//!
//! The monitor listens on a Unix socket for commands from the FinderSync
//! extension (or CLI tools). Commands are text-based, one per line.
//!
//! ## Protocol
//!
//! ```text
//! REFRESH /path/to/folder\n    → queue a re-scan of folder + subfolders, respond "OK\n"
//! REFRESH_ALL\n                 → reload config, queue a full re-scan, respond "OK\n"
//! STATUS\n                      → respond with full status JSON
//! PING\n                        → respond "PONG\n" (health check)
//! ```
//!
//! "OK" means the request is queued, not that the scan finished: the monitor
//! merges repeated requests into one scan and rewrites `status.json` when it
//! completes. Older monitors ran the whole scan before answering, and any
//! monitor answers late while a scan is running, so clients never wait for an
//! answer without a limit: [`UnixSocketClient::request`] and
//! [`UnixSocketClient::notify`] take a timeout, and [`UnixSocketClient::send`]
//! gives up after [`SEND_TIMEOUT`].

use super::IpcConfig;
use crate::errors::AppError;
use std::fmt;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener as TokioUnixListener, UnixStream};
use tracing::{debug, warn};

/// Upper bound on [`UnixSocketClient::send`], so no caller can hang on a
/// monitor that accepted the connection but never answers.
pub const SEND_TIMEOUT: Duration = Duration::from_secs(10);

/// Upper bound on [`nudge_refresh_all`]. A free monitor answers in
/// milliseconds; a busy one reads the request from its socket later.
pub const NUDGE_TIMEOUT: Duration = Duration::from_secs(2);

/// Commands the monitor can receive over the socket.
///
/// The enum name `DaemonCommand` is preserved (not renamed to `MonitorCommand`)
/// because it is purely an internal Rust type; renaming it would be wire-format
/// churn with zero user benefit. The text protocol words (`PING`, `REFRESH`,
/// `STATUS`, `REFRESH_ALL`) are likewise unchanged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DaemonCommand {
    /// Re-scan a specific path and its subfolders.
    Refresh(PathBuf),
    /// Re-scan all monitored paths.
    RefreshAll,
    /// Return the current status JSON.
    Status,
    /// Health check.
    Ping,
    /// Unknown command.
    Unknown(String),
}

impl DaemonCommand {
    /// Parse a command from a text line.
    pub fn parse(line: &str) -> Self {
        let trimmed = line.trim();
        if let Some(path) = trimmed.strip_prefix("REFRESH ") {
            DaemonCommand::Refresh(PathBuf::from(path))
        } else if trimmed == "REFRESH_ALL" {
            DaemonCommand::RefreshAll
        } else if trimmed == "STATUS" {
            DaemonCommand::Status
        } else if trimmed == "PING" {
            DaemonCommand::Ping
        } else {
            DaemonCommand::Unknown(trimmed.to_string())
        }
    }
}

/// Renders the command as its wire text, without the trailing newline.
impl fmt::Display for DaemonCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DaemonCommand::Refresh(path) => write!(f, "REFRESH {}", path.display()),
            DaemonCommand::RefreshAll => f.write_str("REFRESH_ALL"),
            DaemonCommand::Status => f.write_str("STATUS"),
            DaemonCommand::Ping => f.write_str("PING"),
            DaemonCommand::Unknown(line) => f.write_str(line),
        }
    }
}

/// Unix socket listener for the monitor.
pub struct UnixSocketListener {
    path: PathBuf,
}

impl UnixSocketListener {
    /// Creates a new listener for the given socket path.
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    /// The socket path.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Bind and start listening. Removes stale socket file if present.
    pub async fn bind(&self) -> Result<TokioUnixListener, AppError> {
        // Remove stale socket file from a previous run
        if self.path.exists() {
            std::fs::remove_file(&self.path).map_err(|e| {
                AppError::path(format!(
                    "Failed to remove stale socket '{}': {}",
                    self.path.display(),
                    e
                ))
            })?;
        }

        // Ensure parent directory exists
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                AppError::path(format!(
                    "Failed to create socket directory '{}': {}",
                    parent.display(),
                    e
                ))
            })?;
        }

        TokioUnixListener::bind(&self.path).map_err(|e| {
            AppError::path(format!(
                "Failed to bind Unix socket '{}': {}",
                self.path.display(),
                e
            ))
        })
    }

    /// Cleans up the socket file on shutdown.
    pub fn cleanup(&self) {
        if self.path.exists() {
            if let Err(e) = std::fs::remove_file(&self.path) {
                warn!(
                    path = %self.path.display(),
                    error = %e,
                    "Failed to remove socket file during cleanup"
                );
            }
        }
    }
}

/// Read a single command from a connected Unix stream.
pub async fn read_command(stream: &mut BufReader<UnixStream>) -> Result<DaemonCommand, AppError> {
    let mut line = String::new();
    let bytes_read = stream
        .read_line(&mut line)
        .await
        .map_err(|e| AppError::config(format!("Failed to read from socket: {}", e)))?;

    if bytes_read == 0 {
        return Err(AppError::config("Socket connection closed"));
    }

    Ok(DaemonCommand::parse(&line))
}

/// Write a response to a connected Unix stream.
pub async fn write_response(stream: &mut UnixStream, response: &str) -> Result<(), AppError> {
    stream
        .write_all(response.as_bytes())
        .await
        .map_err(|e| AppError::config(format!("Failed to write to socket: {}", e)))?;
    stream
        .flush()
        .await
        .map_err(|e| AppError::config(format!("Failed to flush socket: {}", e)))?;
    Ok(())
}

/// What came back from a [`UnixSocketClient::request`] that reached the monitor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reply {
    /// The monitor's answer line, trailing newline included. Empty when the
    /// monitor closed the connection without answering.
    Answered(String),
    /// No answer within the timeout. The monitor is alive but busy; a request
    /// that was written stays on its socket until the monitor reads it.
    Pending,
}

/// Client for connecting to the monitor's Unix socket.
pub struct UnixSocketClient {
    path: PathBuf,
}

impl UnixSocketClient {
    /// Creates a client targeting the given socket path.
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    /// Send a command and receive the response.
    ///
    /// Fails when the monitor does not answer within [`SEND_TIMEOUT`]. Use
    /// [`Self::request`] to treat a slow monitor as busy instead.
    pub async fn send(&self, command: &str) -> Result<String, AppError> {
        self.send_within(command, SEND_TIMEOUT).await
    }

    async fn send_within(&self, command: &str, timeout: Duration) -> Result<String, AppError> {
        match self.request(command, timeout).await? {
            Reply::Answered(response) => Ok(response),
            Reply::Pending => Err(AppError::config(format!(
                "Monitor did not answer '{command}' within {timeout:?}"
            ))),
        }
    }

    /// Send a command and wait at most `timeout` for the answer.
    ///
    /// A monitor that cannot be reached (no socket, connection refused) is an
    /// error, so callers can still tell "not running" apart. Running out of
    /// time while connecting, writing, or waiting for the answer is
    /// [`Reply::Pending`].
    pub async fn request(&self, command: &str, timeout: Duration) -> Result<Reply, AppError> {
        match tokio::time::timeout(timeout, self.exchange(command)).await {
            Ok(response) => response.map(Reply::Answered),
            Err(_) => {
                debug!(command, ?timeout, "Monitor did not answer in time");
                Ok(Reply::Pending)
            }
        }
    }

    /// Send a command without waiting for its effect: [`Self::request`] with
    /// the answer discarded. Only an unreachable monitor is an error.
    pub async fn notify(&self, command: &str, timeout: Duration) -> Result<(), AppError> {
        self.request(command, timeout).await.map(|_| ())
    }

    /// Connect, write one command line, and read one answer line.
    async fn exchange(&self, command: &str) -> Result<String, AppError> {
        let mut stream = UnixStream::connect(&self.path).await.map_err(|e| {
            AppError::path(format!(
                "Failed to connect to monitor socket '{}': {}",
                self.path.display(),
                e
            ))
        })?;

        // Send command
        let msg = format!("{}\n", command);
        stream
            .write_all(msg.as_bytes())
            .await
            .map_err(|e| AppError::config(format!("Failed to send command: {}", e)))?;
        stream
            .flush()
            .await
            .map_err(|e| AppError::config(format!("Failed to flush: {}", e)))?;

        // Read response
        let mut reader = BufReader::new(stream);
        let mut response = String::new();
        reader
            .read_line(&mut response)
            .await
            .map_err(|e| AppError::config(format!("Failed to read response: {}", e)))?;

        debug!(
            command,
            response = response.trim(),
            "Socket command completed"
        );
        Ok(response)
    }

    /// Ping the monitor. Returns true if it responds.
    pub async fn ping(&self) -> bool {
        match self.send("PING").await {
            Ok(response) => response.trim() == "PONG",
            Err(_) => false,
        }
    }

    /// Request a refresh of a specific path.
    pub async fn refresh(&self, path: &Path) -> Result<String, AppError> {
        self.send(&format!("REFRESH {}", path.display())).await
    }

    /// Request a full refresh of all monitored paths.
    pub async fn refresh_all(&self) -> Result<String, AppError> {
        self.send("REFRESH_ALL").await
    }
}

/// Asks the running monitor to reload its configuration and rescan, without
/// waiting for the scan.
///
/// Best effort and bounded by [`NUDGE_TIMEOUT`], so the commands that nudge
/// after changing repos or the registry never wait on a busy monitor. A
/// monitor that is not running is skipped, and failures are only logged at
/// debug level.
pub async fn nudge_refresh_all() {
    let ipc = match IpcConfig::default_path() {
        Ok(ipc) => ipc,
        Err(e) => {
            debug!(error = %e, "Monitor refresh nudge skipped");
            return;
        }
    };
    let command = DaemonCommand::RefreshAll.to_string();
    let client = UnixSocketClient::new(ipc.socket_path());
    if let Err(e) = client.notify(&command, NUDGE_TIMEOUT).await {
        debug!(error = %e, "Monitor refresh nudge skipped");
    }
}

#[cfg(test)]
#[path = "unix_socket_tests.rs"]
mod tests;
