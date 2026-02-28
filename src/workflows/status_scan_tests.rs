use super::*;
use crate::config::{Config, WorkspaceConfig};

#[test]
fn scan_workspace_status_returns_empty_when_base_path_missing() {
    let config = Config::default();
    let workspace =
        WorkspaceConfig::new_from_root(std::path::Path::new("/tmp/git-same-does-not-exist-xyz"));

    let entries = scan_workspace_status(&config, &workspace);
    assert!(entries.is_empty());
}

#[test]
fn scan_workspace_status_returns_empty_for_empty_directory() {
    let temp = tempfile::tempdir().unwrap();
    let config = Config::default();
    let workspace = WorkspaceConfig::new_from_root(temp.path());

    let entries = scan_workspace_status(&config, &workspace);
    assert!(entries.is_empty());
}
