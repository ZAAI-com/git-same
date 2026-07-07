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
    mirrors: Vec<PathBuf>,
}

impl StatusFileWriter {
    /// Creates a writer for the given status file path.
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            mirrors: Vec::new(),
        }
    }

    /// Creates a writer that, after writing the primary `path`, writes an
    /// identical atomic copy to each path in `mirrors`.
    ///
    /// Used on macOS so the monitor can keep `status.json` in the app-group
    /// container (read by the sandboxed Badges extension) while also mirroring
    /// a real copy into `~/.config/git-same/finder/` that the non-sandboxed
    /// Tauri host can read without crossing the container boundary (which would
    /// trigger the "access data from other apps" TCC prompt).
    pub fn new_with_mirrors(path: PathBuf, mirrors: Vec<PathBuf>) -> Self {
        Self { path, mirrors }
    }

    /// The path this writer writes to.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Writes the status atomically to the primary path and every mirror.
    ///
    /// Each destination is written to a sibling temp file and then renamed, so
    /// readers never observe a partial file and any pre-existing symlink at a
    /// destination is replaced by a real file (rename swaps the directory
    /// entry; it does not follow the link).
    ///
    /// Only a primary-path failure is an error. Mirrors are a convenience copy
    /// for the host app, so a failing mirror (e.g. an unwritable
    /// `~/.config/git-same/finder/`) is logged as a warning and skipped rather
    /// than taking down the caller (the monitor would otherwise crash-loop
    /// under launchd even though the container primary was written fine).
    pub fn write(&self, status: &FinderStatus) -> Result<(), AppError> {
        let json = serde_json::to_string_pretty(status)
            .map_err(|e| AppError::config(format!("Failed to serialize finder status: {}", e)))?;

        write_atomic(&self.path, &json)?;
        for mirror in &self.mirrors {
            if let Err(e) = write_atomic(mirror, &json) {
                tracing::warn!(
                    mirror = %mirror.display(),
                    error = %e,
                    "Failed to write status mirror; primary status file was written"
                );
            }
        }

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

    /// Mirror paths this writer copies to after the primary (test support).
    #[cfg(test)]
    pub(crate) fn mirror_paths(&self) -> &[PathBuf] {
        &self.mirrors
    }
}

/// Removes `path` if it is a symlink, leaving regular files untouched.
///
/// Returns `Ok(true)` when a symlink was removed (or vanished concurrently
/// mid-removal) and `Ok(false)` when there was nothing to remove.
///
/// Used by the Tauri host before reading `status.json`: older layouts
/// symlinked `~/.config/git-same/finder/status.json` into the app-group
/// container, and following that link (via `metadata`/`exists`, which
/// dereference symlinks) would re-trigger the "access data from other apps"
/// TCC prompt on the non-sandboxed host. `symlink_metadata` does not follow
/// the link, so detecting and unlinking it never touches the container; the
/// monitor's next mirror write recreates a real file at the path.
///
/// Concurrent callers may race between the check and the unlink; `NotFound`
/// from the removal is treated as success. The narrower race where the
/// monitor renames a real file over the symlink inside that window is
/// accepted: the next monitor write (at most one scan interval) restores it.
pub fn remove_symlink_if_present(path: &Path) -> Result<bool, AppError> {
    match std::fs::symlink_metadata(path) {
        Ok(meta) if meta.file_type().is_symlink() => match std::fs::remove_file(path) {
            Ok(()) => Ok(true),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(true),
            Err(e) => Err(AppError::path(format!(
                "Failed to remove symlink '{}': {}",
                path.display(),
                e
            ))),
        },
        _ => Ok(false),
    }
}

/// Writes `json` to `path` atomically: write to a sibling `<name>.json.tmp`
/// file, then rename it over `path`. The rename replaces the destination
/// directory entry (including a pre-existing symlink) without following it.
fn write_atomic(path: &Path, json: &str) -> Result<(), AppError> {
    let temp_path = path.with_extension("json.tmp");

    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            AppError::path(format!(
                "Failed to create directory '{}': {}",
                parent.display(),
                e
            ))
        })?;
    }

    // Write to temp file
    std::fs::write(&temp_path, json).map_err(|e| {
        AppError::path(format!(
            "Failed to write temp status file '{}': {}",
            temp_path.display(),
            e
        ))
    })?;

    // Atomic rename
    std::fs::rename(&temp_path, path).map_err(|e| {
        AppError::path(format!(
            "Failed to rename '{}' -> '{}': {}",
            temp_path.display(),
            path.display(),
            e
        ))
    })?;

    Ok(())
}

/// On macOS, ensures `~/.config/git-same/finder/finder.sock` is a symlink
/// pointing into the app-group container directory.
///
/// `status.json` is deliberately **not** symlinked: the monitor writes a real
/// mirror copy there (see [`StatusFileWriter::new_with_mirrors`]) so the
/// non-sandboxed Tauri host can read it without following a link into the
/// container (which would re-trigger the "access data from other apps" prompt).
/// The monitor's first mirror write replaces any leftover `status.json` symlink
/// from an earlier layout with a real file.
///
/// Idempotent. If a legacy regular file already exists at the destination, it
/// is renamed aside as `<name>.user-saved-<UTC-timestamp>` and a `warn` log
/// line is emitted, then the symlink is created. If the legacy directory
/// itself does not exist (fresh install), this is a no-op.
///
/// Pre-existing 3.x users had the monitor writing to `~/.config/git-same/finder/`
/// and the FinderSync extension reading from it via an absolute-path entitlement
/// exception. After Phase B.5, the monitor writes to the group container
/// directly; this helper makes any tool that hardcoded the legacy socket path
/// continue to work via symlink redirection.
#[cfg(target_os = "macos")]
pub fn ensure_legacy_symlinks(group_dir: &Path) -> Result<(), AppError> {
    let legacy_dir = match super::IpcConfig::legacy_default_path() {
        Ok(cfg) => cfg.dir,
        Err(_) => return Ok(()),
    };
    ensure_legacy_symlinks_in(&legacy_dir, group_dir)
}

/// Core of [`ensure_legacy_symlinks`] with the legacy dir passed in, so tests
/// can exercise it against a controlled directory.
#[cfg(target_os = "macos")]
fn ensure_legacy_symlinks_in(legacy_dir: &Path, group_dir: &Path) -> Result<(), AppError> {
    if !legacy_dir.exists() {
        // Fresh install (no XDG config dir at all yet); nothing to migrate.
        return Ok(());
    }

    // Only the socket is symlinked; status.json is a real mirror file written
    // by the monitor (see the doc comment on `ensure_legacy_symlinks`).
    let legacy_sock = legacy_dir.join("finder.sock");
    let target_sock = group_dir.join("finder.sock");
    ensure_one_symlink(&legacy_sock, &target_sock)
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
