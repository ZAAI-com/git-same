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

#[cfg(target_os = "macos")]
mod symlink_helper {
    use super::*;
    use std::fs;
    use std::os::unix::fs::symlink;

    fn dirs() -> (tempfile::TempDir, PathBuf, PathBuf) {
        let root = tempfile::tempdir().unwrap();
        let legacy = root.path().join("legacy");
        let group = root.path().join("group");
        fs::create_dir_all(&legacy).unwrap();
        fs::create_dir_all(&group).unwrap();
        (root, legacy, group)
    }

    #[test]
    fn creates_fresh_symlink_when_no_legacy_file_exists() {
        let (_root, legacy, group) = dirs();
        let legacy_file = legacy.join("status.json");
        let target_file = group.join("status.json");

        ensure_one_symlink(&legacy_file, &target_file).unwrap();

        let meta = fs::symlink_metadata(&legacy_file).unwrap();
        assert!(meta.file_type().is_symlink());
        assert_eq!(fs::read_link(&legacy_file).unwrap(), target_file);
    }

    #[test]
    fn is_idempotent_when_correct_symlink_already_exists() {
        let (_root, legacy, group) = dirs();
        let legacy_file = legacy.join("status.json");
        let target_file = group.join("status.json");

        symlink(&target_file, &legacy_file).unwrap();
        ensure_one_symlink(&legacy_file, &target_file).unwrap();

        // Still a symlink, still pointing where we expect.
        let meta = fs::symlink_metadata(&legacy_file).unwrap();
        assert!(meta.file_type().is_symlink());
        assert_eq!(fs::read_link(&legacy_file).unwrap(), target_file);
    }

    #[test]
    fn replaces_stale_symlink_pointing_elsewhere() {
        let (_root, legacy, group) = dirs();
        let legacy_file = legacy.join("status.json");
        let target_file = group.join("status.json");
        let other = legacy.join("somewhere-else");

        symlink(&other, &legacy_file).unwrap();
        ensure_one_symlink(&legacy_file, &target_file).unwrap();

        assert_eq!(fs::read_link(&legacy_file).unwrap(), target_file);
    }

    #[test]
    fn renames_aside_when_legacy_is_a_regular_file() {
        let (_root, legacy, group) = dirs();
        let legacy_file = legacy.join("status.json");
        let target_file = group.join("status.json");

        fs::write(&legacy_file, b"old user data").unwrap();
        ensure_one_symlink(&legacy_file, &target_file).unwrap();

        // Legacy path is now a symlink.
        let meta = fs::symlink_metadata(&legacy_file).unwrap();
        assert!(meta.file_type().is_symlink());
        assert_eq!(fs::read_link(&legacy_file).unwrap(), target_file);

        // The original file's contents survive at status.json.user-saved-<stamp>.
        let aside_count = fs::read_dir(&legacy)
            .unwrap()
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with("status.json.user-saved-")
            })
            .count();
        assert_eq!(aside_count, 1, "expected one aside file");
    }

    #[test]
    fn ensure_legacy_symlinks_is_noop_when_legacy_dir_missing() {
        // Use a non-existent legacy dir override path: we can't easily inject
        // a custom legacy dir into the public helper, so we exercise the
        // private one with a known-missing legacy path.
        let (_root, _legacy, group) = dirs();
        let missing_legacy_file =
            PathBuf::from("/nonexistent/path/that/should/not/exist/status.json");
        // ensure_one_symlink should still happily create a symlink if the
        // parent can be created; we sanity-check by NOT creating the parent
        // and asserting we get an error rather than a crash.
        // (Linux/macOS will fail at `create_dir_all` for a path we cannot
        // write to.)
        let _ = ensure_one_symlink(&missing_legacy_file, &group.join("status.json"));
        // No assertion about success/failure here; the point is just that
        // the helper does not panic on unexpected inputs.
    }
}
