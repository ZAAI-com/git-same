//! Small filesystem helpers shared across the engine.
//!
//! `atomic_write` is the one place that implements "write a sibling temp file,
//! then rename". Every temp name is unique per process and per call, so two
//! writers (or two tasks in one process) can never clobber each other's
//! in-flight temp file.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Suffix shared by every temp sibling created here. Watchers use it to
/// ignore in-flight writes.
pub const TEMP_SUFFIX: &str = ".tmp";

/// Returns a unique hidden sibling path for `path`.
pub fn temp_sibling(path: &Path) -> PathBuf {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "file".to_string());
    let n = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let temp_name = format!(".{name}.{}.{n}{TEMP_SUFFIX}", std::process::id());
    path.with_file_name(temp_name)
}

/// Atomically replaces `path` with `bytes`, creating parent directories.
///
/// `mode` sets Unix permission bits on the new file (ignored elsewhere).
///
/// The parent directory is synced after the rename. Syncing only the file
/// makes its *contents* durable; it does not make the rename itself durable,
/// and the monitor's install transaction depends on the relative order of two
/// renames surviving a crash.
pub fn atomic_write(path: &Path, bytes: &[u8], mode: Option<u32>) -> std::io::Result<()> {
    let path = resolve_symlink_target(path)?;
    let path = path.as_path();
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty());
    if let Some(parent) = parent {
        std::fs::create_dir_all(parent)?;
    }
    let temp = temp_sibling(path);
    let existing_permissions = std::fs::metadata(path)
        .ok()
        .map(|metadata| metadata.permissions());
    let result = (|| {
        let mut file = std::fs::File::create(&temp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        set_permissions(&temp, mode, existing_permissions)?;
        replace_file(&temp, path)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temp);
        return result;
    }
    sync_dir(parent);
    result
}

/// Follows an existing symlink chain without canonicalising ordinary path
/// components. Atomic replacement must target the file behind a config-file
/// symlink, not replace the symlink itself.
fn resolve_symlink_target(path: &Path) -> std::io::Result<PathBuf> {
    let mut current = path.to_path_buf();
    for _ in 0..40 {
        let metadata = match std::fs::symlink_metadata(&current) {
            Ok(metadata) => metadata,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(current),
            Err(e) => return Err(e),
        };
        if !metadata.file_type().is_symlink() {
            return Ok(current);
        }
        let target = std::fs::read_link(&current)?;
        current = if target.is_absolute() {
            target
        } else {
            current
                .parent()
                .filter(|parent| !parent.as_os_str().is_empty())
                .unwrap_or_else(|| Path::new("."))
                .join(target)
        };
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        "too many symbolic links while resolving atomic-write target",
    ))
}

/// Best-effort durability for a directory entry created by `rename`.
///
/// A failure here means the rename may not survive a power loss, which is
/// strictly better than failing a write that already succeeded. Some
/// filesystems also refuse to open a directory for this purpose at all.
fn sync_dir(parent: Option<&Path>) {
    let Some(parent) = parent else { return };
    if let Ok(dir) = std::fs::File::open(parent) {
        let _ = dir.sync_all();
    }
}

#[cfg(unix)]
fn set_permissions(
    path: &Path,
    mode: Option<u32>,
    existing: Option<std::fs::Permissions>,
) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    match (mode, existing) {
        (Some(mode), _) => std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)),
        (None, Some(permissions)) => std::fs::set_permissions(path, permissions),
        (None, None) => Ok(()),
    }
}

#[cfg(not(unix))]
fn set_permissions(
    _path: &Path,
    _mode: Option<u32>,
    _existing: Option<std::fs::Permissions>,
) -> std::io::Result<()> {
    Ok(())
}

#[cfg(not(windows))]
fn replace_file(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::rename(from, to)
}

#[cfg(windows)]
fn replace_file(from: &Path, to: &Path) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
    };

    let wide = |path: &Path| {
        path.as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect::<Vec<_>>()
    };
    let from = wide(from);
    let to = wide(to);
    // SAFETY: both buffers are NUL-terminated UTF-16 paths and remain alive
    // for the duration of the call.
    if unsafe {
        MoveFileExW(
            from.as_ptr(),
            to.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    } != 0
    {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

#[cfg(test)]
#[path = "fsutil_tests.rs"]
mod tests;
