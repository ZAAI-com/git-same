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

    assert!(cache.is_compatible());

    cache.version = 0;
    assert!(!cache.is_compatible());

    cache.version = CACHE_VERSION + 1;
    assert!(!cache.is_compatible());
}

#[test]
fn test_cache_validity() {
    let repos = HashMap::new();
    let cache = DiscoveryCache::new("testuser".to_string(), repos);

    assert!(cache.is_valid(Duration::from_secs(3600)));

    sleep(Duration::from_millis(1100));
    assert!(!cache.is_valid(Duration::from_secs(1)));
}

#[test]
fn test_cache_age() {
    let repos = HashMap::new();
    let cache = DiscoveryCache::new("testuser".to_string(), repos);

    sleep(Duration::from_millis(100));
    let age = cache.age_secs();
    assert!(age == 0 || age == 1);
}

#[test]
fn test_cache_save_and_load() {
    let temp_dir = TempDir::new().expect("temp dir");
    let cache_path = temp_dir.path().join("workspace-cache.json");

    let manager = CacheManager::with_path(cache_path.clone());

    let mut repos = HashMap::new();
    repos.insert(
        "github".to_string(),
        vec![create_test_repo(1, "repo1", "org1")],
    );

    let cache = DiscoveryCache::new("testuser".to_string(), repos);

    manager.save(&cache).expect("save cache");
    assert!(cache_path.exists());

    let loaded = manager.load().expect("load cache");
    assert!(loaded.is_some());

    let loaded_cache = loaded.expect("cache exists");
    assert_eq!(loaded_cache.username, "testuser");
    assert_eq!(loaded_cache.repo_count, 1);
}

#[test]
fn test_cache_expiration() {
    let temp_dir = TempDir::new().expect("temp dir");
    let cache_path = temp_dir.path().join("workspace-cache.json");

    let manager = CacheManager::with_path(cache_path.clone()).with_ttl(Duration::from_secs(1));

    let repos = HashMap::new();
    let cache = DiscoveryCache::new("testuser".to_string(), repos);

    manager.save(&cache).expect("save cache");

    let loaded = manager.load().expect("load cache");
    assert!(
        loaded.is_some(),
        "Cache should be valid immediately after save"
    );

    let short_ttl_manager =
        CacheManager::with_path(cache_path.clone()).with_ttl(Duration::from_secs(1));
    sleep(Duration::from_millis(1100));

    let loaded = short_ttl_manager.load().expect("load short ttl cache");
    assert!(
        loaded.is_none(),
        "Cache should be expired after waiting longer than TTL"
    );
}

#[test]
fn test_cache_clear() {
    let temp_dir = TempDir::new().expect("temp dir");
    let cache_path = temp_dir.path().join("workspace-cache.json");

    let manager = CacheManager::with_path(cache_path.clone());

    let repos = HashMap::new();
    let cache = DiscoveryCache::new("testuser".to_string(), repos);

    manager.save(&cache).expect("save cache");
    assert!(cache_path.exists());

    manager.clear().expect("clear cache");
    assert!(!cache_path.exists());
}

#[test]
fn test_cache_load_nonexistent() {
    let temp_dir = TempDir::new().expect("temp dir");
    let cache_path = temp_dir.path().join("nonexistent.json");

    let manager = CacheManager::with_path(cache_path);
    let loaded = manager.load().expect("load cache");
    assert!(loaded.is_none());
}
