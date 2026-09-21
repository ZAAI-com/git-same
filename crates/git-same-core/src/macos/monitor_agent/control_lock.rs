//! Control lock: serializes installation, preference changes, and service
//! lifecycle operations across the installer, the app, and the CLI.
//!
//! The file is never removed, because locks are per inode.

use super::system::System;
use crate::errors::MonitorAgentError;
use std::fs::{File, OpenOptions, TryLockError};
use std::path::Path;
use std::time::Duration;

const POLL: Duration = Duration::from_millis(100);

/// How long to wait for a competing operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Wait {
    /// Automatic recovery: never compete with an operation in flight.
    No,
    /// Explicit commands: wait a bounded time, then report busy.
    UpTo(Duration),
}

/// Held for the duration of one lifecycle operation.
#[derive(Debug)]
pub struct ControlLock {
    _file: File,
}

impl ControlLock {
    pub fn acquire(
        path: &Path,
        wait: Wait,
        system: &dyn System,
    ) -> Result<Self, MonitorAgentError> {
        if let Some(parent) = path.parent() {
            create_private_dir(parent)?;
        }
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)
            .map_err(|e| MonitorAgentError::io("Failed to open the monitor control lock", e))?;

        let mut waited = Duration::ZERO;
        loop {
            match file.try_lock() {
                Ok(()) => return Ok(Self { _file: file }),
                Err(TryLockError::WouldBlock) => match wait {
                    Wait::UpTo(limit) if waited < limit => {
                        system.sleep(POLL);
                        waited += POLL;
                    }
                    _ => return Err(MonitorAgentError::Busy),
                },
                Err(TryLockError::Error(e)) => {
                    return Err(MonitorAgentError::io(
                        "Failed to lock the monitor control lock",
                        e,
                    ))
                }
            }
        }
    }
}

/// Creates the managed root as a user-private directory.
pub fn create_private_dir(dir: &Path) -> Result<(), MonitorAgentError> {
    std::fs::create_dir_all(dir)
        .map_err(|e| MonitorAgentError::io(format!("Failed to create '{}'", dir.display()), e))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700)).map_err(|e| {
            MonitorAgentError::io(format!("Failed to restrict '{}'", dir.display()), e)
        })?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "control_lock_tests.rs"]
mod tests;
