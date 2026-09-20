//! Single-instance guard for the monitor process.
//!
//! The runtime lock is held for the whole monitor lifetime, including the
//! initial scan, so at most one monitor ever writes the shared status or owns
//! the socket. Right after taking the lock the monitor writes a runtime
//! identity record next to it. Other processes trust a recorded PID only when
//! the lock is held AND the process start identity does not contradict it: a
//! stale status-file PID alone never identifies a monitor. Contradict, not
//! match -- see [`identity_matches_live_process`] for why an unreadable start
//! identity has to be permissive.
//!
//! The lock file is never removed: deleting it while another process holds
//! its inode would let two monitors each lock a different file.

use super::process;
use crate::fsutil::atomic_write;
use crate::ipc::IpcConfig;
use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions, TryLockError};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// How the monitor process was started.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MonitorMode {
    /// Started by the managed background service.
    Managed,
    /// Started directly by a user (`gisa monitor`).
    Foreground,
}

/// Who holds the runtime lock, written before the initial scan begins.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeIdentity {
    pub pid: u32,
    /// Opaque process start token; `None` where the platform cannot tell.
    pub start_identity: Option<String>,
    pub executable: PathBuf,
    pub mode: MonitorMode,
    /// RFC 3339 timestamp of when the lock was acquired.
    pub started_at: String,
}

/// Observable state of the runtime lock and identity record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeMonitorState {
    Stopped,
    /// The exclusive lock is held while its identity is absent, unreadable,
    /// or temporarily unverifiable. This is a running/starting monitor, never
    /// evidence that the service is stopped.
    HeldUnknown,
    Active(RuntimeIdentity),
}

/// Why the runtime lock could not be acquired.
#[derive(Debug)]
pub enum AcquireError {
    /// Another monitor holds the lock. Carries its identity when readable.
    Held(Option<RuntimeIdentity>),
    Io(std::io::Error),
}

impl std::fmt::Display for AcquireError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AcquireError::Held(Some(identity)) => {
                write!(
                    f,
                    "another monitor is already running (PID {})",
                    identity.pid
                )
            }
            AcquireError::Held(None) => write!(f, "another monitor is already running"),
            AcquireError::Io(e) => write!(f, "could not acquire the monitor runtime lock: {e}"),
        }
    }
}

/// RAII handle on the runtime lock. Dropping it releases the lock and removes
/// the identity record, which is the "monitor exited cleanly" signal.
#[derive(Debug)]
pub struct RuntimeGuard {
    _lock: File,
    identity_path: PathBuf,
    identity: RuntimeIdentity,
}

/// A short probe by another process can hold a shared lock for an instant,
/// so acquisition retries briefly before concluding a monitor is running.
const ACQUIRE_ATTEMPTS: u32 = 5;
const ACQUIRE_BACKOFF: Duration = Duration::from_millis(100);

impl RuntimeGuard {
    /// Takes the runtime lock and writes the identity record.
    pub fn acquire(ipc: &IpcConfig, mode: MonitorMode) -> Result<Self, AcquireError> {
        std::fs::create_dir_all(&ipc.dir).map_err(AcquireError::Io)?;
        let lock = open_lock_file(&ipc.runtime_lock_path()).map_err(AcquireError::Io)?;

        let mut attempt = 0;
        loop {
            match lock.try_lock() {
                Ok(()) => break,
                Err(TryLockError::WouldBlock) => {
                    attempt += 1;
                    if attempt >= ACQUIRE_ATTEMPTS {
                        // Give a holder that released at the retry boundary one
                        // final acquisition attempt before launchd is told a
                        // competing monitor still exists.
                        std::thread::sleep(ACQUIRE_BACKOFF);
                        match lock.try_lock() {
                            Ok(()) => break,
                            Err(TryLockError::WouldBlock) => {
                                return Err(AcquireError::Held(read_identity(ipc)))
                            }
                            Err(TryLockError::Error(e)) => return Err(AcquireError::Io(e)),
                        }
                    }
                    std::thread::sleep(ACQUIRE_BACKOFF);
                }
                Err(TryLockError::Error(e)) => return Err(AcquireError::Io(e)),
            }
        }

        let pid = std::process::id();
        let identity = RuntimeIdentity {
            pid,
            start_identity: process::start_identity(pid),
            executable: std::env::current_exe().unwrap_or_default(),
            mode,
            started_at: chrono::Utc::now().to_rfc3339(),
        };
        let identity_path = ipc.runtime_identity_path();
        let json = serde_json::to_vec_pretty(&identity)
            .map_err(|e| AcquireError::Io(std::io::Error::other(e)))?;
        atomic_write(&identity_path, &json, None).map_err(AcquireError::Io)?;

        Ok(Self {
            _lock: lock,
            identity_path,
            identity,
        })
    }

    /// The identity this guard recorded.
    pub fn identity(&self) -> &RuntimeIdentity {
        &self.identity
    }
}

impl Drop for RuntimeGuard {
    /// Removes the record first, then releases the lock when `_lock` drops.
    ///
    /// Not the other way around: releasing first would let a starting monitor
    /// take the lock and write its own record into the gap, which this drop
    /// would then delete. The reverse gap ("lock held, no record") is read as
    /// "a monitor is running, identity unknown", which is the safe answer.
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.identity_path);
    }
}

/// Returns `true` when some process currently holds the runtime lock.
pub fn lock_is_held(ipc: &IpcConfig) -> bool {
    let path = ipc.runtime_lock_path();
    if !path.exists() {
        return false;
    }
    let Ok(file) = OpenOptions::new().read(true).open(&path) else {
        return false;
    };
    // A shared probe conflicts only with the monitor's exclusive lock, and is
    // released as soon as `file` drops.
    matches!(file.try_lock_shared(), Err(TryLockError::WouldBlock))
}

/// Reads the identity record without validating it.
pub fn read_identity(ipc: &IpcConfig) -> Option<RuntimeIdentity> {
    let content = std::fs::read(ipc.runtime_identity_path()).ok()?;
    serde_json::from_slice(&content).ok()
}

/// Returns the identity of the monitor that is verifiably running right now.
///
/// Requires all of: the runtime lock is held, the record parses, the PID is
/// alive, and the start identity matches when both sides know it.
pub fn active_monitor(ipc: &IpcConfig) -> Option<RuntimeIdentity> {
    match runtime_monitor_state(ipc) {
        RuntimeMonitorState::Active(identity) => Some(identity),
        RuntimeMonitorState::Stopped | RuntimeMonitorState::HeldUnknown => None,
    }
}

/// Probes the lock without collapsing an unreadable identity into Stopped.
pub fn runtime_monitor_state(ipc: &IpcConfig) -> RuntimeMonitorState {
    if !lock_is_held(ipc) {
        return RuntimeMonitorState::Stopped;
    }
    let Some(identity) = read_identity(ipc) else {
        return RuntimeMonitorState::HeldUnknown;
    };
    if identity_matches_live_process(&identity) {
        RuntimeMonitorState::Active(identity)
    } else {
        RuntimeMonitorState::HeldUnknown
    }
}

/// Whether `status.json` was written by this runtime-lock generation.
pub fn scan_complete(ipc: &IpcConfig, identity: &RuntimeIdentity) -> bool {
    let status_path = ipc.status_file_path();
    if !crate::ipc::StatusFileWriter::new(status_path.clone())
        .read()
        .is_ok_and(|status| status.daemon_pid == identity.pid)
    {
        return false;
    }
    let Some(status_modified) = std::fs::metadata(status_path)
        .ok()
        .and_then(|metadata| metadata.modified().ok())
    else {
        return false;
    };
    let Some(identity_modified) = std::fs::metadata(ipc.runtime_identity_path())
        .ok()
        .and_then(|metadata| metadata.modified().ok())
    else {
        // Scripted users may supply an identity without the on-disk guard.
        return true;
    };
    status_modified >= identity_modified
}

/// Returns `true` when `identity` describes the process currently at its PID.
pub fn identity_matches_live_process(identity: &RuntimeIdentity) -> bool {
    if !process::is_alive(identity.pid) {
        return false;
    }
    match (
        &identity.start_identity,
        process::start_identity(identity.pid),
    ) {
        (Some(recorded), Some(current)) => *recorded == current,
        // Deliberately permissive. The remaining arms are "this platform has
        // no start identities" and "the PID is alive but its start time could
        // not be read" (macOS refuses `proc_pidinfo` across uids, and
        // `is_alive` counts EPERM as alive). Answering `false` there would
        // declare a live monitor dead on a transient read failure; the second
        // monitor that followed would lose the lock race and exit 0, which
        // launchd does not retry. The held lock is the proof instead.
        _ => true,
    }
}

fn open_lock_file(path: &Path) -> std::io::Result<File> {
    OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)
}

#[cfg(test)]
#[path = "runtime_guard_tests.rs"]
mod tests;
