//! IPC (Inter-Process Communication) for the daemon and Finder extension.
//!
//! This module provides cross-platform abstractions for:
//! - **Status file**: Atomic JSON writes from the daemon, read by the extension.
//! - **Socket/pipe**: Refresh requests from the extension to the daemon.
//!
//! On macOS/Linux, communication uses Unix domain sockets.
//! On Windows, named pipes are used instead.

pub mod status_file;

#[cfg(unix)]
pub mod unix_socket;

pub use status_file::StatusFileWriter;

#[cfg(unix)]
pub use unix_socket::{UnixSocketClient, UnixSocketListener};

use crate::errors::AppError;
use std::path::PathBuf;

/// IPC configuration paths.
#[derive(Debug, Clone)]
pub struct IpcConfig {
    /// Directory containing IPC files (status.json, finder.sock).
    pub dir: PathBuf,
}

impl IpcConfig {
    /// Creates IPC config pointing to `~/.config/git-same/finder/`.
    pub fn default_path() -> Result<Self, AppError> {
        let config_dir = crate::config::Config::default_path()?;
        // default_path returns .../config.toml, we want .../finder/
        let base_dir = config_dir
            .parent()
            .ok_or_else(|| AppError::config("Could not determine config directory"))?;
        Ok(Self {
            dir: base_dir.join("finder"),
        })
    }

    /// Path to the status JSON file.
    pub fn status_file_path(&self) -> PathBuf {
        self.dir.join("status.json")
    }

    /// Path to the Unix socket (macOS/Linux).
    #[cfg(unix)]
    pub fn socket_path(&self) -> PathBuf {
        self.dir.join("finder.sock")
    }

    /// Path to the preferences JSON file.
    pub fn preferences_path(&self) -> PathBuf {
        self.dir.join("preferences.json")
    }

    /// Ensures the IPC directory exists.
    pub fn ensure_dir(&self) -> Result<(), AppError> {
        std::fs::create_dir_all(&self.dir).map_err(|e| {
            AppError::path(format!(
                "Failed to create IPC directory '{}': {}",
                self.dir.display(),
                e
            ))
        })
    }
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
