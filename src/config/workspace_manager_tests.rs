use super::*;

#[test]
fn dot_dir_is_derived_from_workspace_root() {
    let root = Path::new("/tmp/my-workspace");
    let dot_dir = WorkspaceManager::dot_dir(root);
    assert_eq!(dot_dir, std::path::PathBuf::from("/tmp/my-workspace/.git-same"));
}

#[test]
fn cache_path_is_inside_dot_dir() {
    let root = Path::new("/tmp/my-workspace");
    let cache = WorkspaceManager::cache_path(root);
    assert_eq!(
        cache,
        std::path::PathBuf::from("/tmp/my-workspace/.git-same/cache.json")
    );
}

#[test]
fn sync_history_path_is_inside_dot_dir() {
    let root = Path::new("/tmp/my-workspace");
    let hist = WorkspaceManager::sync_history_path(root);
    assert_eq!(
        hist,
        std::path::PathBuf::from("/tmp/my-workspace/.git-same/sync-history.json")
    );
}
