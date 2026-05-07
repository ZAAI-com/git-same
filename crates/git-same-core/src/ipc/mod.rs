//! IPC (Inter-Process Communication) for the daemon and Finder extension.
//!
//! This module provides cross-platform abstractions for:
//! - **Status file**: Atomic JSON writes from the daemon, read by the extension.
//! - **Socket/pipe**: Refresh requests from the extension to the daemon.
//!
//! On macOS/Linux, communication uses Unix domain sockets.
//! On Windows, named pipes are used instead.
//!
//! ## macOS path resolution
//!
//! On macOS, IPC files live in the app-group container at
//! `~/Library/Group Containers/<APP_GROUP_ID>/` so the sandboxed Badges
//! extension and the (non-sandboxed) Tauri host can both reach them via the
//! `application-groups` entitlement, instead of via per-path absolute-path
//! exceptions that cannot be expanded for arbitrary users.
//!
//! On non-macOS platforms (Linux, Windows), IPC files live under the user's
//! XDG config dir at `~/.config/git-same/finder/`.

pub mod status_file;

#[cfg(unix)]
pub mod unix_socket;

pub use status_file::StatusFileWriter;

#[cfg(unix)]
pub use unix_socket::{UnixSocketClient, UnixSocketListener};

use crate::errors::AppError;
use std::path::PathBuf;

/// App group identifier shared by the daemon, Tauri host, and Badges extension on macOS.
///
/// Apple requires the team-id prefix; `57KL6Y7V32` is the zaai-com Apple Developer team.
/// The Tauri host's `entitlements.plist` and the Badges extension's
/// `GitSameBadges.entitlements` must declare the same value under
/// `com.apple.security.application-groups`.
pub const APP_GROUP_ID: &str = "group.57KL6Y7V32.com.zaai.git-same";

/// IPC configuration paths.
#[derive(Debug, Clone)]
pub struct IpcConfig {
    /// Directory containing IPC files (status.json, finder.sock).
    pub dir: PathBuf,
}

impl IpcConfig {
    /// Returns the platform-default IPC config.
    ///
    /// On macOS, this is `~/Library/Group Containers/<APP_GROUP_ID>/`.
    /// On other platforms (and on macOS when `$HOME` is unavailable), this is
    /// the legacy `~/.config/git-same/finder/`.
    pub fn default_path() -> Result<Self, AppError> {
        #[cfg(target_os = "macos")]
        {
            if let Some(group_dir) = macos_group_container_dir() {
                return Ok(Self { dir: group_dir });
            }
            // Fall through to legacy if HOME is unset (test environments).
        }
        Self::legacy_default_path()
    }

    /// Returns the legacy `~/.config/git-same/finder/` path.
    ///
    /// Used as the macOS fallback and as the source side of legacy-symlink
    /// migration on macOS (see `status_file::ensure_legacy_symlinks`).
    pub fn legacy_default_path() -> Result<Self, AppError> {
        let config_dir = crate::config::Config::default_path()?;
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

/// Resolves the macOS app-group container directory based on `$HOME`.
///
/// Returns `None` when `$HOME` is unset (typically only inside tests). The
/// directory itself is created on demand by `IpcConfig::ensure_dir`; it does
/// NOT need to pre-exist for this function to return `Some`.
#[cfg(target_os = "macos")]
pub(crate) fn macos_group_container_dir() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    Some(
        PathBuf::from(home)
            .join("Library")
            .join("Group Containers")
            .join(APP_GROUP_ID),
    )
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
