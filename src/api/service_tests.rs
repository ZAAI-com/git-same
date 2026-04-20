use super::*;
use crate::api::AmbientUpgradeCache;
use crate::config::Config;
use crate::git::traits::mock::{MockConfig, MockGit};
use crate::git::traits::RepoStatus;
use crate::types::finder_status::{Badge, FinderRepoStatus};

fn default_config() -> Config {
    // Tests should not trigger the ambient $HOME walk, so disable it unless
    // a specific test opts in.
    let mut cfg = Config::default();
    cfg.finder.show_ambient = false;
    cfg
}

#[test]
fn test_scan_repo_clean() {
    let mock = MockGit::new();
    let config = default_config();
    let service = RepoScanService::new(&mock, &config);

    let status = service.scan_repo(Path::new("/tmp/repo"), Some("ws"), Some("org"));

    assert_eq!(status.badge, Badge::Green);
    assert_eq!(status.current_branch, "main");
    assert_eq!(status.staged_count, 0);
    assert_eq!(status.unstaged_count, 0);
    assert_eq!(status.workspace, Some("ws".to_string()));
    assert_eq!(status.org, Some("org".to_string()));
    assert!(status.all_branches_synced);
}

#[test]
fn test_scan_repo_dirty() {
    let mock_cfg = MockConfig {
        default_status: RepoStatus {
            branch: "feature".to_string(),
            is_uncommitted: true,
            ahead: 2,
            behind: 0,
            has_untracked: true,
            staged_count: 1,
            unstaged_count: 3,
            untracked_count: 2,
        },
        ..Default::default()
    };
    let mock = MockGit::with_config(mock_cfg);
    let config = default_config();
    let service = RepoScanService::new(&mock, &config);

    let status = service.scan_repo(Path::new("/tmp/repo"), None, None);

    assert_eq!(status.badge, Badge::Red);
    assert_eq!(status.current_branch, "feature");
    assert_eq!(status.staged_count, 1);
    assert_eq!(status.unstaged_count, 3);
    assert_eq!(status.untracked_count, 2);
    assert_eq!(status.ahead, 2);
}

#[test]
fn test_scan_repo_no_workspace() {
    let mock = MockGit::new();
    let config = default_config();
    let service = RepoScanService::new(&mock, &config);

    let status = service.scan_repo(Path::new("/tmp/repo"), None, None);

    assert!(status.workspace.is_none());
    assert!(status.org.is_none());
}

#[test]
fn test_check_important_ignored_none() {
    let mock = MockGit::new();
    let config = default_config();
    let service = RepoScanService::new(&mock, &config);

    let (has, files) = service.check_important_ignored(Path::new("/tmp/repo"));
    assert!(!has);
    assert!(files.is_empty());
}

#[test]
fn test_scan_all_empty_workspaces() {
    let mock = MockGit::new();
    let config = default_config();
    let service = RepoScanService::new(&mock, &config);

    let status = service.scan_all(12345).unwrap();
    assert_eq!(status.daemon_pid, 12345);
    assert!(status.workspaces.is_empty());
    assert!(status.repos.is_empty());
    assert!(status.org_folders.is_empty());
}

#[test]
fn scan_ambient_repo_is_minimal_and_gray() {
    let mock = MockGit::new();
    let config = default_config();
    let service = RepoScanService::new(&mock, &config);

    let status = service.scan_ambient_repo(Path::new("/tmp/ambient"));

    assert_eq!(status.badge, Badge::Gray);
    assert_eq!(status.path, std::path::PathBuf::from("/tmp/ambient"));
    assert!(status.workspace.is_none());
    assert!(status.org.is_none());
    assert_eq!(status.staged_count, 0);
    assert_eq!(status.commit_count, 0);
    assert!(status.branches.is_empty());
    assert!(status.remotes.is_empty());
}

#[test]
fn scan_all_emits_ambient_gray_repos_when_enabled() {
    let tmp = tempfile::TempDir::new().unwrap();
    std::fs::create_dir_all(tmp.path().join("alpha/.git")).unwrap();
    std::fs::create_dir_all(tmp.path().join("beta/.git")).unwrap();
    std::fs::create_dir_all(tmp.path().join("not-a-repo")).unwrap();

    let mock = MockGit::new();
    let mut cfg = Config::default();
    cfg.finder.show_ambient = true;
    cfg.finder.scan_roots = vec![tmp.path().to_string_lossy().to_string()];
    cfg.finder.max_depth = 2;
    cfg.finder.exclude_dirs = Vec::new();

    let service = RepoScanService::new(&mock, &cfg);
    let status = service.scan_all(1).unwrap();

    let gray_count = status
        .repos
        .iter()
        .filter(|r| r.badge == Badge::Gray)
        .count();
    assert_eq!(gray_count, 2);
    assert!(status
        .repos
        .iter()
        .any(|r| r.path.ends_with("alpha") && r.badge == Badge::Gray));
    assert!(status
        .repos
        .iter()
        .any(|r| r.path.ends_with("beta") && r.badge == Badge::Gray));
    let canonical_tmp = std::fs::canonicalize(tmp.path()).unwrap();
    assert!(status.monitored_roots.iter().any(|p| p == &canonical_tmp));
}

#[test]
fn ambient_upgrade_cache_preserves_semantic_color() {
    let tmp = tempfile::TempDir::new().unwrap();
    std::fs::create_dir_all(tmp.path().join("myrepo/.git")).unwrap();
    let repo_path = std::fs::canonicalize(tmp.path().join("myrepo")).unwrap();

    let mock = MockGit::new();
    let mut cfg = Config::default();
    cfg.finder.show_ambient = true;
    cfg.finder.scan_roots = vec![tmp.path().to_string_lossy().to_string()];
    cfg.finder.max_depth = 2;
    cfg.finder.exclude_dirs = Vec::new();

    let upgrades = AmbientUpgradeCache::new();
    // Prime the cache with a Green upgraded entry.
    let upgraded = FinderRepoStatus {
        path: repo_path.clone(),
        workspace: None,
        org: None,
        badge: Badge::Green,
        current_branch: "main".to_string(),
        default_branch: None,
        commit_count: 42,
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
    };
    upgrades.set(repo_path.clone(), upgraded);

    let service = RepoScanService::new(&mock, &cfg).with_ambient_upgrades(upgrades);
    let status = service.scan_all(1).unwrap();

    let emitted = status
        .repos
        .iter()
        .find(|r| std::fs::canonicalize(&r.path).unwrap_or(r.path.clone()) == repo_path)
        .expect("ambient repo should be emitted");
    assert_eq!(emitted.badge, Badge::Green);
    assert_eq!(emitted.commit_count, 42);
}
