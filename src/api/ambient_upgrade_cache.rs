//! In-memory cache of full-status entries for ambient (non-workspace) repos.
//!
//! Ambient repos start with `Badge::Gray`. When the user right-clicks a gray
//! repo, the extension sends `REFRESH /path` over the socket. The daemon then
//! runs a full `scan_repo` for that path and stores the result here. On every
//! subsequent `scan_all`, ambient entries found in this cache are emitted with
//! their full semantic badge instead of reverting to gray.
//!
//! The cache is not persisted to disk: it lives only for the current daemon
//! run. Restarting the daemon returns all ambient repos to gray until the user
//! opens their context menus again.

use crate::types::finder_status::FinderRepoStatus;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

#[derive(Clone, Default)]
pub struct AmbientUpgradeCache {
    inner: Arc<Mutex<HashMap<PathBuf, FinderRepoStatus>>>,
}

impl AmbientUpgradeCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, path: &Path) -> Option<FinderRepoStatus> {
        self.inner.lock().ok()?.get(path).cloned()
    }

    pub fn set(&self, path: PathBuf, status: FinderRepoStatus) {
        if let Ok(mut guard) = self.inner.lock() {
            guard.insert(path, status);
        }
    }

    pub fn remove(&self, path: &Path) {
        if let Ok(mut guard) = self.inner.lock() {
            guard.remove(path);
        }
    }
}

#[cfg(test)]
#[path = "ambient_upgrade_cache_tests.rs"]
mod tests;
