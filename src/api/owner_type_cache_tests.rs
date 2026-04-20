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
