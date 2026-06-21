//! Unix domain socket IPC for macOS and Linux.
//!
//! The monitor listens on a Unix socket for commands from the FinderSync
//! extension (or CLI tools). Commands are text-based, one per line.
//!
//! ## Protocol
//!
//! ```text
//! REFRESH /path/to/folder\n    → re-scan folder + subfolders, respond "OK\n"
//! REFRESH_ALL\n                 → re-scan everything, respond "OK\n"
//! STATUS\n                      → respond with full status JSON
//! PING\n                        → respond "PONG\n" (health check)
//! ```

use crate::errors::AppError;
use std::path::{Path, PathBuf};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener as TokioUnixListener, UnixStream};
use tracing::{debug, warn};

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
    pub async fn send(&self, command: &str) -> Result<String, AppError> {
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

#[cfg(test)]
#[path = "unix_socket_tests.rs"]
mod tests;
