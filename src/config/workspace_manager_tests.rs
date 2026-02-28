use super::*;

#[test]
fn dot_dir_is_derived_from_workspace_root() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("my-workspace");
    std::fs::create_dir_all(&root).unwrap();
    let dot_dir = WorkspaceManager::dot_dir(&root);
    assert_eq!(dot_dir, root.join(".git-same"));
}

#[test]
fn cache_path_is_inside_dot_dir() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("my-workspace");
    std::fs::create_dir_all(&root).unwrap();
    let cache = WorkspaceManager::cache_path(&root);
    assert_eq!(cache, root.join(".git-same/cache.json"));
}

#[test]
fn sync_history_path_is_inside_dot_dir() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("my-workspace");
    std::fs::create_dir_all(&root).unwrap();
    let hist = WorkspaceManager::sync_history_path(&root);
    assert_eq!(hist, root.join(".git-same/sync-history.json"));
}
