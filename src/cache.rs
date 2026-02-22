//! Discovery cache module
//!
//! Caches GitHub API discovery results to avoid hitting rate limits
//! and speed up subsequent runs.

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

/// Discovery cache data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryCache {
    /// Cache format version for forward compatibility.
    /// If missing during deserialization, defaults to 0 (pre-versioned cache).
    #[serde(default)]
    pub version: u32,

    /// When the discovery was last performed (Unix timestamp)
    pub last_discovery: u64,

    /// Username or identifier
    pub username: String,

    /// List of organization names
    pub orgs: Vec<String>,

    /// Total number of repositories discovered
    pub repo_count: usize,

    /// Cached repositories by provider
    pub repos: HashMap<String, Vec<OwnedRepo>>,
}

impl DiscoveryCache {
    /// Create a new cache entry
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
            .unwrap()
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

    /// Check if the cache is still valid
    pub fn is_valid(&self, ttl: Duration) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        if now < self.last_discovery {
            return false;
        }

        let age = now - self.last_discovery;
        age < ttl.as_secs()
    }

    /// Get the age of the cache in seconds
    pub fn age_secs(&self) -> u64 {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        now.saturating_sub(self.last_discovery)
    }
}

/// Cache manager
pub struct CacheManager {
    cache_path: PathBuf,
    ttl: Duration,
}

impl CacheManager {
    /// Create a new cache manager with default cache path
    pub fn new() -> Result<Self> {
        let cache_path = Self::default_cache_path()?;
        Ok(Self {
            cache_path,
            ttl: DEFAULT_CACHE_TTL,
        })
    }

    /// Create a cache manager with a custom path
    pub fn with_path(cache_path: PathBuf) -> Self {
        Self {
            cache_path,
            ttl: DEFAULT_CACHE_TTL,
        }
    }

    /// Create a cache manager with a custom TTL
    pub fn with_ttl(mut self, ttl: Duration) -> Self {
        self.ttl = ttl;
        self
    }

    /// Get the default cache path (~/.config/git-same/cache.json)
    pub fn default_cache_path() -> Result<PathBuf> {
        #[cfg(target_os = "macos")]
        let config_dir = {
            let home = std::env::var("HOME").context("HOME environment variable not set")?;
            PathBuf::from(home).join(".config").join("git-same")
        };
        #[cfg(not(target_os = "macos"))]
        let config_dir = if let Some(dir) = directories::ProjectDirs::from("", "", "git-same") {
            dir.config_dir().to_path_buf()
        } else {
            let home = std::env::var("HOME").context("HOME environment variable not set")?;
            PathBuf::from(home).join(".config").join("git-same")
        };

        Ok(config_dir.join("cache.json"))
    }

    /// Load the cache if it exists and is valid
    pub fn load(&self) -> Result<Option<DiscoveryCache>> {
        if !self.cache_path.exists() {
            debug!(path = %self.cache_path.display(), "Cache file does not exist");
            return Ok(None);
        }

        let content = fs::read_to_string(&self.cache_path).context("Failed to read cache file")?;

        let cache: DiscoveryCache =
            serde_json::from_str(&content).context("Failed to parse cache file")?;

        // Check version compatibility
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

    /// Save the cache to disk
    pub fn save(&self, cache: &DiscoveryCache) -> Result<()> {
        // Ensure parent directory exists
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

    /// Clear the cache file
    pub fn clear(&self) -> Result<()> {
        if self.cache_path.exists() {
            fs::remove_file(&self.cache_path).context("Failed to remove cache file")?;
        }
        Ok(())
    }

    /// Get the cache path
    pub fn path(&self) -> &Path {
        &self.cache_path
    }
}

impl Default for CacheManager {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| {
            // Fallback to temp directory if we can't determine config dir
            Self::with_path(std::env::temp_dir().join("git-same-cache.json"))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Repo;
    use std::thread::sleep;
    use tempfile::TempDir;

    fn create_test_repo(id: u64, name: &str, owner: &str) -> OwnedRepo {
        OwnedRepo {
            owner: owner.to_string(),
            repo: Repo {
                id,
                name: name.to_string(),
                full_name: format!("{}/{}", owner, name),
                ssh_url: format!("git@github.com:{}/{}.git", owner, name),
                clone_url: format!("https://github.com/{}/{}.git", owner, name),
                default_branch: "main".to_string(),
                private: false,
                archived: false,
                fork: false,
                pushed_at: None,
                description: None,
            },
        }
    }

    #[test]
    fn test_cache_creation() {
        let mut repos = HashMap::new();
        repos.insert(
            "github".to_string(),
            vec![
                create_test_repo(1, "repo1", "org1"),
                create_test_repo(2, "repo2", "org2"),
            ],
        );

        let cache = DiscoveryCache::new("testuser".to_string(), repos);

        assert_eq!(cache.version, CACHE_VERSION);
        assert_eq!(cache.username, "testuser");
        assert_eq!(cache.repo_count, 2);
        assert_eq!(cache.orgs.len(), 2);
        assert!(cache.orgs.contains(&"org1".to_string()));
        assert!(cache.orgs.contains(&"org2".to_string()));
        assert!(cache.is_compatible());
    }

    #[test]
    fn test_cache_version_compatibility() {
        let repos = HashMap::new();
        let mut cache = DiscoveryCache::new("testuser".to_string(), repos);

        // Current version should be compatible
        assert!(cache.is_compatible());

        // Old version should not be compatible
        cache.version = 0;
        assert!(!cache.is_compatible());

        // Future version should not be compatible
        cache.version = CACHE_VERSION + 1;
        assert!(!cache.is_compatible());
    }

    #[test]
    fn test_cache_validity() {
        let repos = HashMap::new();
        let cache = DiscoveryCache::new("testuser".to_string(), repos);

        // Should be valid immediately
        assert!(cache.is_valid(Duration::from_secs(3600)));

        // Test with very short TTL
        sleep(Duration::from_millis(100));
        assert!(!cache.is_valid(Duration::from_millis(50)));
    }

    #[test]
    fn test_cache_age() {
        let repos = HashMap::new();
        let cache = DiscoveryCache::new("testuser".to_string(), repos);

        sleep(Duration::from_millis(100));
        let age = cache.age_secs();
        assert!(age == 0 || age == 1); // Should be very recent
    }

    #[test]
    fn test_cache_save_and_load() {
        let temp_dir = TempDir::new().unwrap();
        let cache_path = temp_dir.path().join("cache.json");

        let manager = CacheManager::with_path(cache_path.clone());

        let mut repos = HashMap::new();
        repos.insert(
            "github".to_string(),
            vec![create_test_repo(1, "repo1", "org1")],
        );

        let cache = DiscoveryCache::new("testuser".to_string(), repos);

        // Save cache
        manager.save(&cache).unwrap();
        assert!(cache_path.exists());

        // Load cache
        let loaded = manager.load().unwrap();
        assert!(loaded.is_some());

        let loaded_cache = loaded.unwrap();
        assert_eq!(loaded_cache.username, "testuser");
        assert_eq!(loaded_cache.repo_count, 1);
    }

    #[test]
    fn test_cache_expiration() {
        let temp_dir = TempDir::new().unwrap();
        let cache_path = temp_dir.path().join("cache.json");

        // Use a generous TTL to ensure cache is valid when first loaded
        let manager = CacheManager::with_path(cache_path.clone()).with_ttl(Duration::from_secs(1));

        let repos = HashMap::new();
        let cache = DiscoveryCache::new("testuser".to_string(), repos);

        manager.save(&cache).unwrap();

        // Cache should be valid well within TTL
        let loaded = manager.load().unwrap();
        assert!(
            loaded.is_some(),
            "Cache should be valid immediately after save"
        );

        // Now test with a very short TTL to ensure expiration works
        let short_ttl_manager =
            CacheManager::with_path(cache_path.clone()).with_ttl(Duration::from_millis(50));

        // Wait long enough to definitely expire
        sleep(Duration::from_millis(100));

        // Cache should be expired with short TTL
        let loaded = short_ttl_manager.load().unwrap();
        assert!(
            loaded.is_none(),
            "Cache should be expired after waiting longer than TTL"
        );
    }

    #[test]
    fn test_cache_clear() {
        let temp_dir = TempDir::new().unwrap();
        let cache_path = temp_dir.path().join("cache.json");

        let manager = CacheManager::with_path(cache_path.clone());

        let repos = HashMap::new();
        let cache = DiscoveryCache::new("testuser".to_string(), repos);

        manager.save(&cache).unwrap();
        assert!(cache_path.exists());

        manager.clear().unwrap();
        assert!(!cache_path.exists());
    }

    #[test]
    fn test_cache_load_nonexistent() {
        let temp_dir = TempDir::new().unwrap();
        let cache_path = temp_dir.path().join("nonexistent.json");

        let manager = CacheManager::with_path(cache_path);

        let loaded = manager.load().unwrap();
        assert!(loaded.is_none());
    }
}
