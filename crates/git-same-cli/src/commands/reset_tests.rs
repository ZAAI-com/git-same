use super::*;

#[test]
fn test_reset_target_is_empty_when_nothing_exists() {
    let target = ResetTarget {
        config_dir: PathBuf::from("/nonexistent"),
        config_file: None,
        workspaces: Vec::new(),
    };
    assert!(target.is_empty());
}

#[test]
fn test_reset_target_not_empty_with_config() {
    let target = ResetTarget {
        config_dir: PathBuf::from("/some/dir"),
        config_file: Some(PathBuf::from("/some/dir/config.toml")),
        workspaces: Vec::new(),
    };
    assert!(!target.is_empty());
}

#[test]
fn test_reset_target_not_empty_with_workspaces() {
    let target = ResetTarget {
        config_dir: PathBuf::from("/some/dir"),
        config_file: None,
        workspaces: vec![WorkspaceDetail {
            root_path: PathBuf::from("/tmp/ws1"),
            orgs: vec!["org1".to_string()],
            last_synced: None,
            dot_dir: PathBuf::from("/tmp/ws1/.git-same"),
            cache_size: None,
        }],
    };
    assert!(!target.is_empty());
}

#[test]
fn test_humanize_timestamp_hours() {
    let ts = (Utc::now() - chrono::Duration::hours(3)).to_rfc3339();
    assert_eq!(humanize_timestamp(&ts), "3h ago");
}

#[test]
fn test_humanize_timestamp_days() {
    let ts = (Utc::now() - chrono::Duration::days(5)).to_rfc3339();
    assert_eq!(humanize_timestamp(&ts), "5d ago");
}

#[test]
fn test_humanize_timestamp_invalid() {
    assert_eq!(humanize_timestamp("not-a-date"), "not-a-date");
}

#[test]
fn test_humanize_timestamp_just_now() {
    let ts = Utc::now().to_rfc3339();
    assert_eq!(humanize_timestamp(&ts), "just now");
}

#[test]
fn test_format_bytes() {
    assert_eq!(format_bytes(500), "500 B");
    assert_eq!(format_bytes(1024), "1.0 KB");
    assert_eq!(format_bytes(15360), "15.0 KB");
    assert_eq!(format_bytes(1_048_576), "1.0 MB");
}

#[test]
fn test_display_workspace_detail_no_panic() {
    let ws = WorkspaceDetail {
        root_path: PathBuf::from("/tmp/test"),
        orgs: vec!["org1".to_string(), "org2".to_string()],
        last_synced: Some("2026-02-24T10:00:00Z".to_string()),
        dot_dir: PathBuf::from("/tmp/test/.git-same"),
        cache_size: Some(12345),
    };
    let output = Output::new(git_same_core::output::Verbosity::Quiet, false);
    display_workspace_detail(&ws, &output);
}

#[test]
fn test_display_detailed_targets_everything() {
    let target = ResetTarget {
        config_dir: PathBuf::from("/tmp/test"),
        config_file: Some(PathBuf::from("/tmp/test/config.toml")),
        workspaces: vec![WorkspaceDetail {
            root_path: PathBuf::from("/tmp/ws1"),
            orgs: Vec::new(),
            last_synced: None,
            dot_dir: PathBuf::from("/tmp/ws1/.git-same"),
            cache_size: None,
        }],
    };
    let output = Output::new(git_same_core::output::Verbosity::Quiet, false);
    display_detailed_targets(&ResetScope::Everything, &target, &output);
}

#[test]
fn test_display_detailed_targets_config_only() {
    let target = ResetTarget {
        config_dir: PathBuf::from("/tmp/test"),
        config_file: Some(PathBuf::from("/tmp/test/config.toml")),
        workspaces: Vec::new(),
    };
    let output = Output::new(git_same_core::output::Verbosity::Quiet, false);
    display_detailed_targets(&ResetScope::ConfigOnly, &target, &output);
}

#[cfg(target_os = "macos")]
#[test]
fn test_remove_workspace_dir_clears_folder_icon() {
    use git_same_core::macos::folder_icon;

    let temp = tempfile::tempdir().expect("tempdir");
    let root = temp.path().to_path_buf();

    // Paint the workspace folder icon directly so the test doesn't depend on
    // the full setup path.
    folder_icon::set(&root, folder_icon::WORKSPACE_FOLDER_ICNS)
        .expect("set icon on tempdir should succeed");
    assert!(folder_icon::is_set(&root), "icon was not set");

    // dot_dir intentionally does not exist: remove_dir_all will fail and
    // remove_workspace_dir will return false without ever touching the
    // global registry — but only after clear_or_log has run, which is what
    // we're verifying.
    let ws = WorkspaceDetail {
        root_path: root.clone(),
        orgs: Vec::new(),
        last_synced: None,
        dot_dir: root.join(".git-same-does-not-exist"),
        cache_size: None,
    };
    let output = Output::new(git_same_core::output::Verbosity::Quiet, false);
    let _ = remove_workspace_dir(&ws, &output);

    assert!(
        !folder_icon::is_set(&root),
        "remove_workspace_dir did not clear the workspace folder icon"
    );
}
