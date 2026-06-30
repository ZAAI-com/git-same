use super::*;
use crate::types::finder_status::Badge;
use std::path::PathBuf;

fn sample_status(path: &str, badge: Badge) -> FinderRepoStatus {
    FinderRepoStatus {
        path: PathBuf::from(path),
        workspace: None,
        org: None,
        badge,
        current_branch: "main".to_string(),
        default_branch: None,
        commit_count: 0,
        staged_count: 0,
        unstaged_count: 0,
        untracked_count: 0,
        ahead: 0,
        behind: 0,
        stash_count: 0,
        has_important_ignored_files: false,
        important_ignored_files: Vec::new(),
        branches: Vec::new(),
        all_branches_synced: true,
        remotes: Vec::new(),
        worktrees: Vec::new(),
        all_worktrees_synced: true,
        read_error: None,
    }
}

#[test]
fn set_then_get_returns_stored_entry() {
    let cache = AmbientUpgradeCache::new();
    let path = PathBuf::from("/tmp/repo-a");
    cache.set(path.clone(), sample_status("/tmp/repo-a", Badge::Green));

    let got = cache.get(&path).unwrap();
    assert_eq!(got.badge, Badge::Green);
    assert_eq!(got.path, path);
}

#[test]
fn get_missing_path_returns_none() {
    let cache = AmbientUpgradeCache::new();
    assert!(cache.get(&PathBuf::from("/tmp/not-there")).is_none());
}

#[test]
fn remove_drops_entry() {
    let cache = AmbientUpgradeCache::new();
    let path = PathBuf::from("/tmp/repo-b");
    cache.set(path.clone(), sample_status("/tmp/repo-b", Badge::Red));
    cache.remove(&path);
    assert!(cache.get(&path).is_none());
}

#[test]
fn clone_shares_storage() {
    let cache = AmbientUpgradeCache::new();
    let handle = cache.clone();
    let path = PathBuf::from("/tmp/shared");
    handle.set(path.clone(), sample_status("/tmp/shared", Badge::Orange));

    // Original handle sees the write.
    assert!(cache.get(&path).is_some());
}
