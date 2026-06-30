//! Atomic JSON status file writer and reader.
//!
//! The monitor writes the status file atomically by writing to a temporary
//! file first, then renaming it. This ensures the FinderSync extension
//! never reads a partial/corrupt file.

use crate::errors::AppError;
use crate::types::finder_status::FinderStatus;
use std::path::{Path, PathBuf};

/// Writes and reads the Finder status JSON file atomically.
#[derive(Debug, Clone)]
pub struct StatusFileWriter {
    path: PathBuf,
}

impl StatusFileWriter {
    /// Creates a writer for the given status file path.
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    /// The path this writer writes to.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Writes the status atomically (write to temp, then rename).
    pub fn write(&self, status: &FinderStatus) -> Result<(), AppError> {
        let json = serde_json::to_string_pretty(status)
            .map_err(|e| AppError::config(format!("Failed to serialize finder status: {}", e)))?;

        let temp_path = self.path.with_extension("json.tmp");

        // Ensure parent directory exists
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                AppError::path(format!(
                    "Failed to create directory '{}': {}",
                    parent.display(),
                    e
                ))
            })?;
        }

        // Write to temp file
        std::fs::write(&temp_path, &json).map_err(|e| {
            AppError::path(format!(
                "Failed to write temp status file '{}': {}",
                temp_path.display(),
                e
            ))
        })?;

        // Atomic rename
        std::fs::rename(&temp_path, &self.path).map_err(|e| {
            AppError::path(format!(
                "Failed to rename '{}' → '{}': {}",
                temp_path.display(),
                self.path.display(),
                e
            ))
        })?;

        Ok(())
    }

    /// Reads and parses the status file.
    pub fn read(&self) -> Result<FinderStatus, AppError> {
        let content = std::fs::read_to_string(&self.path).map_err(|e| {
            AppError::path(format!(
                "Failed to read status file '{}': {}",
                self.path.display(),
                e
            ))
        })?;

        serde_json::from_str(&content)
            .map_err(|e| AppError::config(format!("Failed to parse status file: {}", e)))
    }

    /// Checks if the status file exists.
    pub fn exists(&self) -> bool {
        self.path.exists()
    }
}

/// On macOS, ensures `~/.config/git-same/finder/{status.json, finder.sock}` are
/// symlinks pointing into the app-group container directory.
///
/// Idempotent. If a legacy regular file already exists at the destination, it
/// is renamed aside as `<name>.user-saved-<UTC-timestamp>` and a `warn` log
/// line is emitted, then the symlink is created. If the legacy directory
/// itself does not exist (fresh install), this is a no-op.
///
/// Pre-existing 3.x users had the monitor writing to `~/.config/git-same/finder/`
/// and the FinderSync extension reading from it via an absolute-path entitlement
/// exception. After Phase B.5, the monitor writes to the group container
/// directly; this helper makes any tool that hardcoded the legacy path
/// continue to work via symlink redirection.
#[cfg(target_os = "macos")]
pub fn ensure_legacy_symlinks(group_dir: &Path) -> Result<(), AppError> {
    let legacy_dir = match super::IpcConfig::legacy_default_path() {
        Ok(cfg) => cfg.dir,
        Err(_) => return Ok(()),
    };

    if !legacy_dir.exists() {
        // Fresh install (no XDG config dir at all yet); nothing to migrate.
        return Ok(());
    }

    for filename in &["status.json", "finder.sock"] {
        let legacy_path = legacy_dir.join(filename);
        let target_path = group_dir.join(filename);
        ensure_one_symlink(&legacy_path, &target_path)?;
    }

    Ok(())
}

/// Non-macOS no-op so the monitor can call this unconditionally without `cfg`
/// gates at the call site.
#[cfg(not(target_os = "macos"))]
pub fn ensure_legacy_symlinks(_group_dir: &Path) -> Result<(), AppError> {
    Ok(())
}

#[cfg(target_os = "macos")]
fn ensure_one_symlink(legacy_path: &Path, target_path: &Path) -> Result<(), AppError> {
    use std::os::unix::fs::symlink;

    match std::fs::symlink_metadata(legacy_path) {
        Ok(meta) if meta.file_type().is_symlink() => {
            if std::fs::read_link(legacy_path).ok().as_deref() == Some(target_path) {
                return Ok(());
            }
            std::fs::remove_file(legacy_path).map_err(|e| {
                AppError::path(format!(
                    "Failed to remove stale symlink '{}': {}",
                    legacy_path.display(),
                    e
                ))
            })?;
        }
        Ok(_) => {
            let aside = aside_path(legacy_path);
            std::fs::rename(legacy_path, &aside).map_err(|e| {
                AppError::path(format!(
                    "Failed to rename legacy file '{}' to '{}': {}",
                    legacy_path.display(),
                    aside.display(),
                    e
                ))
            })?;
            tracing::warn!(
                legacy = %legacy_path.display(),
                aside = %aside.display(),
                target = %target_path.display(),
                "Renamed legacy regular file aside; replacing with symlink to group container"
            );
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            // No legacy file; just create the symlink.
        }
        Err(e) => {
            return Err(AppError::path(format!(
                "Failed to inspect '{}': {}",
                legacy_path.display(),
                e
            )));
        }
    }

    if let Some(parent) = legacy_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            AppError::path(format!(
                "Failed to create legacy parent dir '{}': {}",
                parent.display(),
                e
            ))
        })?;
    }

    symlink(target_path, legacy_path).map_err(|e| {
        AppError::path(format!(
            "Failed to symlink '{}' -> '{}': {}",
            legacy_path.display(),
            target_path.display(),
            e
        ))
    })?;
    Ok(())
}

#[cfg(target_os = "macos")]
fn aside_path(path: &Path) -> PathBuf {
    use std::ffi::OsString;
    let stamp = chrono::Utc::now().format("%Y%m%dT%H%M%SZ").to_string();
    let mut name = OsString::from(path.as_os_str());
    name.push(format!(".user-saved-{}", stamp));
    PathBuf::from(name)
}

#[cfg(test)]
#[path = "status_file_tests.rs"]
mod tests;
