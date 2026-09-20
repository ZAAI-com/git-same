//! Shared, reloadable configuration snapshot for the monitor loop.
//!
//! A running monitor must pick up registry and config changes without a
//! restart: automatic recovery deliberately leaves a healthy monitor alone,
//! so a restart can never be the transport for "I registered a workspace".
//! Reloads happen on `REFRESH_ALL` and when the file's `(mtime, len)` stamp
//! changed before a periodic full scan. A failed reload keeps the previous
//! snapshot: a syntax error in the file must not take badges down.

use crate::config::Config;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;
use tracing::{info, warn};

type Stamp = (SystemTime, u64);

/// Cloneable handle on the monitor's current configuration.
#[derive(Debug, Clone)]
pub struct LiveConfig {
    current: Arc<Mutex<Arc<Config>>>,
    /// Source file. `None` means the config is fixed for the process lifetime.
    path: Option<PathBuf>,
    stamp: Arc<Mutex<Option<Stamp>>>,
}

impl LiveConfig {
    /// Wraps `config`, reloading from `path` when one is given.
    ///
    /// The caller loaded `config` at some earlier moment, so stamping the file
    /// now would adopt a stamp that is newer than the snapshot: a write landing
    /// in between would be invisible forever, and "I registered a workspace"
    /// would never reach the monitor. The stamp is therefore taken *before*
    /// re-reading, and the fresh read wins when it succeeds. Erring the other
    /// way is safe: a stamp older than the snapshot costs one redundant reload.
    pub fn new(config: Config, path: Option<PathBuf>) -> Self {
        let stamp = path.as_deref().and_then(file_stamp);
        let config = match path.as_deref() {
            // A file that no longer parses is not a reason to refuse to start;
            // the caller's already-validated copy stands in.
            Some(path) => Config::load_from(path).unwrap_or(config),
            None => config,
        };
        Self {
            current: Arc::new(Mutex::new(Arc::new(config))),
            path,
            stamp: Arc::new(Mutex::new(stamp)),
        }
    }

    /// The configuration to use for the next piece of work.
    pub fn snapshot(&self) -> Arc<Config> {
        self.current
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// Reloads when the file stamp changed. Returns `true` if the snapshot
    /// was replaced.
    pub fn reload_if_changed(&self) -> bool {
        let Some(path) = self.path.as_deref() else {
            return false;
        };
        let stamp = file_stamp(path);
        {
            let mut known = self.stamp.lock().unwrap_or_else(|e| e.into_inner());
            if *known == stamp {
                return false;
            }
            // Remember the stamp even when the load below fails, so a broken
            // file is reported once instead of on every scan.
            *known = stamp;
        }
        match Config::load_from(path) {
            Ok(config) => {
                *self.current.lock().unwrap_or_else(|e| e.into_inner()) = Arc::new(config);
                info!(path = %path.display(), "Configuration reloaded");
                true
            }
            Err(e) => {
                warn!(error = %e, "Configuration reload failed; keeping the previous configuration");
                false
            }
        }
    }
}

fn file_stamp(path: &Path) -> Option<Stamp> {
    let metadata = std::fs::metadata(path).ok()?;
    Some((metadata.modified().ok()?, metadata.len()))
}

#[cfg(test)]
#[path = "live_config_tests.rs"]
mod tests;
