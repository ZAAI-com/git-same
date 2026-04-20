use super::*;
use crate::config::Config;
use crate::git::traits::mock::{MockConfig, MockGit};
use crate::git::traits::RepoStatus;
use crate::types::finder_status::Badge;

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
