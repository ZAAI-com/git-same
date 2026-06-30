use crate::types::OwnedRepo;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::{debug, warn};

/// Default cache TTL (1 hour)
const DEFAULT_CACHE_TTL: Duration = Duration::from_secs(3600);

/// Current cache format version.
/// Increment this when making breaking changes to the cache format.
pub const CACHE_VERSION: u32 = 1;

/// Discovery cache data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryCache {
    /// Cache format version for forward compatibility.
    /// If missing during deserialization, defaults to 0 (pre-versioned cache).
    #[serde(default)]
    pub version: u32,

    /// When the discovery was last performed (Unix timestamp).
    pub last_discovery: u64,

    /// Username or identifier.
    pub username: String,

    /// List of organization names.
    pub orgs: Vec<String>,

    /// Total number of repositories discovered.
    pub repo_count: usize,

    /// Cached repositories by provider.
    pub repos: HashMap<String, Vec<OwnedRepo>>,
}

impl DiscoveryCache {
    /// Create a new cache entry.
    pub fn new(username: String, repos: HashMap<String, Vec<OwnedRepo>>) -> Self {
        let orgs: Vec<String> = repos
            .values()
            .flat_map(|r| r.iter().map(|owned| owned.owner.clone()))
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        let repo_count = repos.values().map(|r| r.len()).sum();

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        debug!(
            version = CACHE_VERSION,
            repo_count,
            org_count = orgs.len(),
            "Creating new discovery cache"
        );

        Self {
            version: CACHE_VERSION,
            last_discovery: now,
            username,
            orgs,
            repo_count,
            repos,
        }
    }

    /// Check if this cache is compatible with the current version.
    pub fn is_compatible(&self) -> bool {
        self.version == CACHE_VERSION
    }

    /// Check if the cache is still valid.
    pub fn is_valid(&self, ttl: Duration) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        if now < self.last_discovery {
            return false;
        }

        let age = now - self.last_discovery;
        age < ttl.as_secs()
    }

    /// Get the age of the cache in seconds.
    pub fn age_secs(&self) -> u64 {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        now.saturating_sub(self.last_discovery)
    }
}

/// Discovery cache manager.
pub struct CacheManager {
    cache_path: PathBuf,
    ttl: Duration,
}

impl CacheManager {
    /// Create a cache manager for a specific workspace root path.
    ///
    /// Cache is persisted at `<workspace-root>/.git-same/cache.json`.
    pub fn for_workspace(root: &Path) -> Result<Self> {
        let cache_path = crate::config::WorkspaceStore::cache_path(root);
        Ok(Self {
            cache_path,
            ttl: DEFAULT_CACHE_TTL,
        })
    }

    /// Create a cache manager with a custom path.
    pub fn with_path(cache_path: PathBuf) -> Self {
        Self {
            cache_path,
            ttl: DEFAULT_CACHE_TTL,
        }
    }

    /// Create a cache manager with a custom TTL.
    pub fn with_ttl(mut self, ttl: Duration) -> Self {
        self.ttl = ttl;
        self
    }

    /// Load the cache if it exists and is valid.
    pub fn load(&self) -> Result<Option<DiscoveryCache>> {
        if !self.cache_path.exists() {
            debug!(path = %self.cache_path.display(), "Cache file does not exist");
            return Ok(None);
        }

        let content = match fs::read_to_string(&self.cache_path) {
            Ok(content) => content,
            Err(err) => {
                warn!(
                    path = %self.cache_path.display(),
                    error = %err,
                    "Cache file unreadable, ignoring cache"
                );
                return Ok(None);
            }
        };
        let cache: DiscoveryCache = match serde_json::from_str(&content) {
            Ok(cache) => cache,
            Err(err) => {
                warn!(
                    path = %self.cache_path.display(),
                    error = %err,
                    "Cache file malformed, ignoring cache"
                );
                return Ok(None);
            }
        };

        if !cache.is_compatible() {
            warn!(
                cache_version = cache.version,
                current_version = CACHE_VERSION,
                "Cache version mismatch, ignoring stale cache"
            );
            return Ok(None);
        }

        if cache.is_valid(self.ttl) {
            debug!(
                age_secs = cache.age_secs(),
                repo_count = cache.repo_count,
                "Loaded valid cache"
            );
            Ok(Some(cache))
        } else {
            debug!(age_secs = cache.age_secs(), "Cache expired");
            Ok(None)
        }
    }

    /// Save the cache to disk.
    pub fn save(&self, cache: &DiscoveryCache) -> Result<()> {
        if let Some(parent) = self.cache_path.parent() {
            fs::create_dir_all(parent).context("Failed to create cache directory")?;
        }

        let json = serde_json::to_string_pretty(cache).context("Failed to serialize cache")?;
        fs::write(&self.cache_path, &json).context("Failed to write cache file")?;

        debug!(
            path = %self.cache_path.display(),
            version = cache.version,
            repo_count = cache.repo_count,
            bytes = json.len(),
            "Saved cache to disk"
        );

        Ok(())
    }

    /// Clear the cache file.
    pub fn clear(&self) -> Result<()> {
        if self.cache_path.exists() {
            fs::remove_file(&self.cache_path).context("Failed to remove cache file")?;
        }
        Ok(())
    }

    /// Get the cache path.
    pub fn path(&self) -> &Path {
        &self.cache_path
    }
}

#[cfg(test)]
#[path = "discovery_tests.rs"]
mod tests;
