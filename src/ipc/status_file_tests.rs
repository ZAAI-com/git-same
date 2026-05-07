use super::*;
use crate::types::finder_status::{
    Badge, FinderBranchInfo, FinderRepoStatus, FinderStatus, FinderWorkspaceInfo,
};
use std::path::PathBuf;

fn sample_status() -> FinderStatus {
    let mut status = FinderStatus::new(12345, "2026-04-04T10:30:00Z".to_string());
    status.workspaces.push(FinderWorkspaceInfo {
        name: "github".to_string(),
        root: PathBuf::from("/Users/test/repos"),
        orgs: vec!["zaai-com".to_string()],
    });
    status.repos.push(FinderRepoStatus {
        path: PathBuf::from("/Users/test/repos/zaai-com/git-same"),
        workspace: Some("github".to_string()),
        org: Some("zaai-com".to_string()),
        badge: Badge::Green,
        current_branch: "main".to_string(),
        default_branch: Some("main".to_string()),
        commit_count: 847,
        staged_count: 0,
        unstaged_count: 0,
        untracked_count: 0,
        ahead: 0,
        behind: 0,
        stash_count: 0,
        has_important_ignored_files: false,
        important_ignored_files: Vec::new(),
        branches: vec![FinderBranchInfo {
            name: "main".to_string(),
            upstream: Some("origin/main".to_string()),
            ahead: 0,
            behind: 0,
            synced: true,
        }],
        all_branches_synced: true,
        remotes: vec![],
        worktrees: Vec::new(),
        all_worktrees_synced: true,
        read_error: None,
    });
    status
}

#[test]
fn test_write_and_read_roundtrip() {
    let temp = tempfile::tempdir().unwrap();
    let writer = StatusFileWriter::new(temp.path().join("status.json"));

    let status = sample_status();
    writer.write(&status).unwrap();

    assert!(writer.exists());

    let read_back = writer.read().unwrap();
    assert_eq!(read_back, status);
}

#[test]
fn test_write_creates_parent_dirs() {
    let temp = tempfile::tempdir().unwrap();
    let writer = StatusFileWriter::new(temp.path().join("sub/dir/status.json"));

    let status = FinderStatus::new(1, "now".to_string());
    writer.write(&status).unwrap();

    assert!(writer.exists());
}

#[test]
fn test_write_overwrites_existing() {
    let temp = tempfile::tempdir().unwrap();
    let writer = StatusFileWriter::new(temp.path().join("status.json"));

    let status1 = FinderStatus::new(1, "first".to_string());
    writer.write(&status1).unwrap();

    let status2 = FinderStatus::new(2, "second".to_string());
    writer.write(&status2).unwrap();

    let read_back = writer.read().unwrap();
    assert_eq!(read_back.daemon_pid, 2);
    assert_eq!(read_back.timestamp, "second");
}

#[test]
fn test_read_nonexistent_file() {
    let writer = StatusFileWriter::new(PathBuf::from("/nonexistent/status.json"));
    assert!(!writer.exists());
    assert!(writer.read().is_err());
}

#[test]
fn test_no_temp_file_remains_after_write() {
    let temp = tempfile::tempdir().unwrap();
    let writer = StatusFileWriter::new(temp.path().join("status.json"));

    let status = FinderStatus::new(1, "now".to_string());
    writer.write(&status).unwrap();

    // The .tmp file should not exist after atomic rename
    let temp_path = temp.path().join("status.json.tmp");
    assert!(!temp_path.exists());
}
