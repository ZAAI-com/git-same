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
    let parent = path.parent();
    if let Some(parent) = parent {
        std::fs::create_dir_all(parent)?;
    }
    let temp = temp_sibling(path);
    let result = (|| {
        let mut file = std::fs::File::create(&temp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        set_mode(&temp, mode)?;
        std::fs::rename(&temp, path)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temp);
        return result;
    }
    sync_dir(parent);
    result
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
fn set_mode(path: &Path, mode: Option<u32>) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    match mode {
        Some(mode) => std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)),
        None => Ok(()),
    }
}

#[cfg(not(unix))]
fn set_mode(_path: &Path, _mode: Option<u32>) -> std::io::Result<()> {
    Ok(())
}

#[cfg(test)]
#[path = "fsutil_tests.rs"]
mod tests;
