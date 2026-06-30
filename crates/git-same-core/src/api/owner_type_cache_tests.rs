use super::*;
use tempfile::TempDir;

#[test]
fn load_empty_when_file_missing() {
    let dir = TempDir::new().unwrap();
    let cache = OwnerTypeCache::load(dir.path().join("owner_types.json"));
    assert!(cache.get("nobody").is_none());
}

#[test]
fn set_and_get_roundtrips() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("owner_types.json");
    let cache = OwnerTypeCache::load(path.clone());
    cache.set("alice", OwnerType::User).unwrap();
    cache.set("acme", OwnerType::Organization).unwrap();
    assert_eq!(cache.get("alice"), Some(OwnerType::User));
    assert_eq!(cache.get("acme"), Some(OwnerType::Organization));

    let reloaded = OwnerTypeCache::load(path);
    assert_eq!(reloaded.get("alice"), Some(OwnerType::User));
    assert_eq!(reloaded.get("acme"), Some(OwnerType::Organization));
}

#[test]
fn missing_returns_unknown_names() {
    let dir = TempDir::new().unwrap();
    let cache = OwnerTypeCache::load(dir.path().join("owner_types.json"));
    cache.set("known", OwnerType::User).unwrap();
    let todo = cache.missing(["known", "unseen-a", "unseen-b"]);
    assert_eq!(todo, vec!["unseen-a".to_string(), "unseen-b".to_string()]);
}

#[test]
fn operations_recover_from_poisoned_mutex() {
    use std::sync::Arc;

    let dir = TempDir::new().unwrap();
    let cache = OwnerTypeCache::load(dir.path().join("owner_types.json"));
    cache.set("alice", OwnerType::User).unwrap();

    // Poison the mutex by panicking while holding the lock.
    let inner = Arc::clone(&cache.inner);
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _guard = inner.lock().unwrap();
        panic!("intentional poison");
    }));
    assert!(inner.is_poisoned(), "mutex should be poisoned");

    // Every public operation must still work despite the poison.
    assert_eq!(cache.get("alice"), Some(OwnerType::User));
    assert_eq!(cache.missing(["alice", "bob"]), vec!["bob".to_string()]);
    cache.set("bob", OwnerType::Organization).unwrap();
    assert_eq!(cache.get("bob"), Some(OwnerType::Organization));
}
