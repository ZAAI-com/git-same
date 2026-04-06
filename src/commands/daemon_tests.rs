use super::*;
use crate::git::traits::mock::{MockConfig, MockGit};
use crate::git::traits::RepoStatus;
use crate::types::finder_status::Badge;

#[test]
fn test_scan_single_repo_clean() {
    let mock = MockGit::new();
    let status = scan_single_repo(&mock, Path::new("/tmp/repo"), Some("ws"), Some("org"));

    assert_eq!(status.badge, Badge::Green);
    assert_eq!(status.current_branch, "main");
    assert_eq!(status.staged_count, 0);
    assert_eq!(status.unstaged_count, 0);
    assert_eq!(status.workspace, Some("ws".to_string()));
    assert_eq!(status.org, Some("org".to_string()));
    assert!(status.all_branches_synced);
}

#[test]
fn test_scan_single_repo_dirty() {
    let config = MockConfig {
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
    let mock = MockGit::with_config(config);
    let status = scan_single_repo(&mock, Path::new("/tmp/repo"), None, None);

    assert_eq!(status.badge, Badge::Red);
    assert_eq!(status.current_branch, "feature");
    assert_eq!(status.staged_count, 1);
    assert_eq!(status.unstaged_count, 3);
    assert_eq!(status.untracked_count, 2);
    assert_eq!(status.ahead, 2);
}

#[test]
fn test_scan_single_repo_no_workspace() {
    let mock = MockGit::new();
    let status = scan_single_repo(&mock, Path::new("/tmp/repo"), None, None);

    assert!(status.workspace.is_none());
    assert!(status.org.is_none());
}

#[test]
fn test_is_process_alive_self() {
    let pid = std::process::id();
    assert!(is_process_alive(pid));
}

#[test]
fn test_is_process_alive_nonexistent() {
    // PID 99999 is very unlikely to exist
    assert!(!is_process_alive(99999));
}

#[test]
fn test_check_important_ignored_files_none() {
    let mock = MockGit::new();
    let (has, files) = check_important_ignored_files(&mock, Path::new("/tmp/repo"));
    assert!(!has);
    assert!(files.is_empty());
}
