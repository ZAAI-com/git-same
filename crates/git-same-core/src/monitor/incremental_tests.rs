use super::*;
use crate::api::RepoScanService;
use crate::config::Config;
use crate::git::traits::mock::MockGit;
use crate::types::finder_status::{Badge, FinderStatus, FinderWorkspaceInfo};

fn make_status_with_workspace(name: &str, root: &Path) -> FinderStatus {
    let mut status = FinderStatus::new(0, "0".to_string());
    status.workspaces.push(FinderWorkspaceInfo {
        name: name.to_string(),
        root: root.to_path_buf(),
        orgs: Vec::new(),
    });
    status
}

#[test]
fn rescan_inserts_new_repo_entry_with_derived_labels() {
    let temp = tempfile::tempdir().unwrap();
    let workspace_root = temp.path().to_path_buf();
    let repo_path = workspace_root.join("acme/widgets");
    std::fs::create_dir_all(repo_path.join(".git")).unwrap();

    let mock = MockGit::new();
    let mut cfg = Config::default();
    cfg.finder.show_ambient = false;
    let service = RepoScanService::new(&mock, &cfg);

    let mut status = make_status_with_workspace("workspace", &workspace_root);

    let changed = rescan_and_merge(&service, &mut status, &repo_path);

    assert!(changed);
    let canonical = std::fs::canonicalize(&repo_path).unwrap();
    let entry = status
        .repos
        .iter()
        .find(|r| r.path == canonical)
        .expect("repo should be present");
    assert_eq!(entry.workspace.as_deref(), Some("workspace"));
    assert_eq!(entry.org.as_deref(), Some("acme"));
    assert_eq!(entry.badge, Badge::Green);
}

#[test]
fn rescan_returns_false_when_nothing_changed() {
    let temp = tempfile::tempdir().unwrap();
    let workspace_root = temp.path().to_path_buf();
    let repo_path = workspace_root.join("acme/widgets");
    std::fs::create_dir_all(repo_path.join(".git")).unwrap();

    let mock = MockGit::new();
    let mut cfg = Config::default();
    cfg.finder.show_ambient = false;
    let service = RepoScanService::new(&mock, &cfg);

    let mut status = make_status_with_workspace("workspace", &workspace_root);

    assert!(rescan_and_merge(&service, &mut status, &repo_path));
    let timestamp_after_first = status.timestamp.clone();
    assert!(!rescan_and_merge(&service, &mut status, &repo_path));
    assert_eq!(status.timestamp, timestamp_after_first);
}

#[test]
fn rescan_removes_entry_when_repo_is_gone() {
    let temp = tempfile::tempdir().unwrap();
    let workspace_root = temp.path().to_path_buf();
    let repo_path = workspace_root.join("acme/widgets");
    std::fs::create_dir_all(repo_path.join(".git")).unwrap();

    let mock = MockGit::new();
    let mut cfg = Config::default();
    cfg.finder.show_ambient = false;
    let service = RepoScanService::new(&mock, &cfg);

    let mut status = make_status_with_workspace("workspace", &workspace_root);
    rescan_and_merge(&service, &mut status, &repo_path);

    std::fs::remove_dir_all(&repo_path).unwrap();

    let changed = rescan_and_merge(&service, &mut status, &repo_path);
    assert!(changed);
    assert!(status.repos.is_empty());
}
