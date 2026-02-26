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
            name: "ws1".to_string(),
            base_path: "~/github".to_string(),
            orgs: vec!["org1".to_string()],
            last_synced: None,
            dir: PathBuf::from("/some/dir/ws1"),
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
        name: "test".to_string(),
        base_path: "~/github".to_string(),
        orgs: vec!["org1".to_string(), "org2".to_string()],
        last_synced: Some("2026-02-24T10:00:00Z".to_string()),
        dir: PathBuf::from("/tmp/test"),
        cache_size: Some(12345),
    };
    let output = Output::new(crate::output::Verbosity::Quiet, false);
    display_workspace_detail(&ws, &output);
}

#[test]
fn test_display_detailed_targets_everything() {
    let target = ResetTarget {
        config_dir: PathBuf::from("/tmp/test"),
        config_file: Some(PathBuf::from("/tmp/test/config.toml")),
        workspaces: vec![WorkspaceDetail {
            name: "ws1".to_string(),
            base_path: "~/github".to_string(),
            orgs: Vec::new(),
            last_synced: None,
            dir: PathBuf::from("/tmp/test/ws1"),
            cache_size: None,
        }],
    };
    let output = Output::new(crate::output::Verbosity::Quiet, false);
    display_detailed_targets(&ResetScope::Everything, &target, &output);
}

#[test]
fn test_display_detailed_targets_config_only() {
    let target = ResetTarget {
        config_dir: PathBuf::from("/tmp/test"),
        config_file: Some(PathBuf::from("/tmp/test/config.toml")),
        workspaces: Vec::new(),
    };
    let output = Output::new(crate::output::Verbosity::Quiet, false);
    display_detailed_targets(&ResetScope::ConfigOnly, &target, &output);
}
