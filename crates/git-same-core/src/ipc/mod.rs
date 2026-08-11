//! IPC (Inter-Process Communication) for the monitor and Finder extension.
//!
//! This module provides cross-platform abstractions for:
//! - **Status file**: Atomic JSON writes from the monitor, read by the extension.
//! - **Socket/pipe**: Refresh requests from the extension to the monitor.
//!
//! On macOS/Linux, communication uses Unix domain sockets.
//! On Windows, named pipes are used instead.
//!
//! ## macOS path resolution
//!
//! On macOS, IPC files live in the app-group container at
//! `~/Library/Group Containers/<APP_GROUP_ID>/` so the sandboxed Badges
//! extension can reach them via the `application-groups` entitlement, instead
//! of via per-path absolute-path exceptions that cannot be expanded for
//! arbitrary users.
//!
//! The non-sandboxed Tauri host deliberately does NOT read from the container:
//! for a non-sandboxed process, reaching into an app container triggers the
//! "access data from other apps" TCC prompt. Instead the monitor mirrors a
//! real `status.json` into the host-facing dir from
//! [`IpcConfig::host_status_path`] (`~/.config/git-same/finder/`), and only
//! `finder.sock` is symlinked there (see `status_file::ensure_legacy_symlinks`).
//!
//! On non-macOS platforms (Linux, Windows), IPC files live under the user's
//! XDG config dir at `~/.config/git-same/finder/`.

pub mod status_file;

#[cfg(unix)]
pub mod unix_socket;

pub use status_file::{remove_symlink_if_present, StatusFileWriter};

#[cfg(unix)]
pub use unix_socket::{UnixSocketClient, UnixSocketListener};

use crate::errors::AppError;
use std::path::PathBuf;

/// App group identifier shared by the monitor, Tauri host, and Badges extension on macOS.
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
    /// migration on macOS (see `status_file::ensure_legacy_symlinks`). This is
    /// the same directory as [`Self::host_status_path`], which is the
    /// host-facing name for it; hosts reading live status should use that name.
    pub fn legacy_default_path() -> Result<Self, AppError> {
        let config_dir = crate::config::Config::default_path()?;
        let base_dir = config_dir
            .parent()
            .ok_or_else(|| AppError::config("Could not determine config directory"))?;
        Ok(Self {
            dir: base_dir.join("finder"),
        })
    }

    /// Returns the host-facing, non-container IPC dir (`~/.config/git-same/finder/`).
    ///
    /// On macOS the monitor mirrors a real `status.json` here so the
    /// non-sandboxed Tauri host can read live status without reaching into the
    /// app-group container, which would trigger the "access data from other
    /// apps" TCC prompt. This is the same directory as
    /// [`Self::legacy_default_path`]: the distinct name documents the
    /// host-facing role, while the legacy name documents its role as the
    /// source side of the symlink migration.
    pub fn host_status_path() -> Result<Self, AppError> {
        Self::legacy_default_path()
    }

    /// Path to the status JSON file.
    pub fn status_file_path(&self) -> PathBuf {
        self.dir.join("status.json")
    }

    /// Returns the status writer for this config, with the platform's mirror
    /// policy applied.
    ///
    /// On macOS, when this config points at the app-group container (the
    /// monitor's primary location), the writer also mirrors `status.json`
    /// into the host-facing dir from [`Self::host_status_path`] so the
    /// non-sandboxed Tauri host can read live status without crossing the
    /// container boundary (which would trigger the "access data from other
    /// apps" TCC prompt). Custom directories (tests, embedders) and other
    /// platforms get a plain, mirror-less writer, so a caller-supplied dir
    /// never leaks writes into the real user's host dir.
    pub fn status_writer(&self) -> StatusFileWriter {
        let primary = self.status_file_path();
        #[cfg(target_os = "macos")]
        {
            if Some(self.dir.as_path()) == macos_group_container_dir().as_deref() {
                if let Ok(host) = Self::host_status_path() {
                    let mirror = host.status_file_path();
                    if mirror != primary {
                        return StatusFileWriter::new_with_mirrors(primary, vec![mirror]);
                    }
                }
            }
        }
        StatusFileWriter::new(primary)
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
