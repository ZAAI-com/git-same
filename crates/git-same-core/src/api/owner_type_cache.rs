//! File-backed cache of GitHub owner classifications.
//!
//! Used by the Finder badge daemon so that `OrgFolderInfo.owner_type` can be
//! populated without hitting the GitHub API on every scan.

use crate::errors::Result;
use crate::types::OwnerType;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// JSON-backed map of `name -> OwnerType`.
#[derive(Clone)]
pub struct OwnerTypeCache {
    path: PathBuf,
    inner: Arc<Mutex<HashMap<String, OwnerType>>>,
}

impl OwnerTypeCache {
    /// Creates a new cache at the given path and loads existing entries if
    /// the file exists. Missing or unreadable files yield an empty cache
    /// without error: classification is best-effort.
    pub fn load(path: PathBuf) -> Self {
        let map = std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str::<HashMap<String, OwnerType>>(&s).ok())
            .unwrap_or_default();
        Self {
            path,
            inner: Arc::new(Mutex::new(map)),
        }
    }

    /// Returns the cached owner type, or `None` if not yet classified.
    pub fn get(&self, name: &str) -> Option<OwnerType> {
        self.inner.lock().ok()?.get(name).copied()
    }

    /// Inserts or updates a cache entry and persists to disk.
    pub fn set(&self, name: &str, owner_type: OwnerType) -> Result<()> {
        {
            let mut guard = self.inner.lock().unwrap();
            guard.insert(name.to_string(), owner_type);
        }
        self.persist()
    }

    /// Names with no entry in the cache (targets for classification).
    pub fn missing<'a>(&self, names: impl IntoIterator<Item = &'a str>) -> Vec<String> {
        let guard = self.inner.lock().unwrap();
        names
            .into_iter()
            .filter(|n| !guard.contains_key(*n))
            .map(|n| n.to_string())
            .collect()
    }

    fn persist(&self) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let snapshot: HashMap<String, OwnerType> = self.inner.lock().unwrap().clone();
        let tmp = self.path.with_extension("json.tmp");
        let data = serde_json::to_vec_pretty(&snapshot)
            .map_err(|e| std::io::Error::other(format!("serialize cache: {e}")))?;
        std::fs::write(&tmp, data)?;
        std::fs::rename(&tmp, &self.path)?;
        Ok(())
    }

    /// Default path under the Finder IPC directory.
    pub fn default_path(finder_dir: &Path) -> PathBuf {
        finder_dir.join("owner_types.json")
    }
}

#[cfg(test)]
#[path = "owner_type_cache_tests.rs"]
mod tests;
