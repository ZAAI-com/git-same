use super::*;
use std::path::Path;

#[test]
fn dot_dir_cache_and_config_paths_are_derived_from_root() {
    let root = Path::new("/tmp/my-workspace");

    let dot_dir = WorkspaceStore::dot_dir(root);
    let config = WorkspaceStore::config_path(root);
    let cache = WorkspaceStore::cache_path(root);
    let history = WorkspaceStore::sync_history_path(root);

    assert_eq!(
        dot_dir,
        std::path::PathBuf::from("/tmp/my-workspace/.git-same")
    );
    assert_eq!(
        config,
        std::path::PathBuf::from("/tmp/my-workspace/.git-same/config.toml")
    );
    assert_eq!(
        cache,
        std::path::PathBuf::from("/tmp/my-workspace/.git-same/cache.json")
    );
    assert_eq!(
        history,
        std::path::PathBuf::from("/tmp/my-workspace/.git-same/sync-history.json")
    );
}

#[test]
fn load_returns_error_when_no_config_exists() {
    let temp = tempfile::tempdir().unwrap();
    let err = WorkspaceStore::load(temp.path()).unwrap_err();
    assert!(err.to_string().contains("No workspace config found"));
}

#[test]
fn save_creates_dot_dir_and_config_file() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("my-ws");
    std::fs::create_dir_all(&root).unwrap();

    let ws = WorkspaceConfig::new_from_root(&root);
    WorkspaceStore::save(&ws).unwrap();

    assert!(WorkspaceStore::config_path(&root).exists());
}

#[test]
fn load_from_path_roundtrip_sets_root_path_from_parent() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("roundtrip");
    let dot_dir = root.join(".git-same");
    std::fs::create_dir_all(&dot_dir).unwrap();

    let ws = WorkspaceConfig::new_from_root(&root);
    let config_path = dot_dir.join("config.toml");
    std::fs::write(&config_path, ws.to_toml().unwrap()).unwrap();

    let loaded = WorkspaceStore::load_from_path(&config_path).unwrap();
    // root_path is canonicalized, so compare the file name component
    assert_eq!(loaded.root_path.file_name(), root.file_name());
}

#[test]
fn delete_returns_error_when_dot_dir_missing() {
    let temp = tempfile::tempdir().unwrap();
    // No .git-same/ directory inside temp, so delete should fail
    let err = WorkspaceStore::delete(temp.path()).unwrap_err();
    assert!(err.to_string().contains("No workspace config found"));
}
