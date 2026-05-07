use super::*;

#[test]
fn resolve_from_list_errors_when_no_workspaces() {
    let err = WorkspacePolicy::resolve_from_list(Vec::new()).unwrap_err();
    assert!(err.to_string().contains("No workspaces configured"));
}

#[test]
fn resolve_from_list_returns_single_workspace() {
    let ws = WorkspaceConfig::new_from_root(std::path::Path::new("/tmp/solo"));
    let resolved = WorkspacePolicy::resolve_from_list(vec![ws.clone()]).unwrap();
    assert_eq!(resolved.root_path, std::path::PathBuf::from("/tmp/solo"));
}

#[test]
fn resolve_from_list_errors_when_multiple_workspaces() {
    let ws1 = WorkspaceConfig::new_from_root(std::path::Path::new("/tmp/a"));
    let ws2 = WorkspaceConfig::new_from_root(std::path::Path::new("/tmp/b"));

    let err = WorkspacePolicy::resolve_from_list(vec![ws1, ws2]).unwrap_err();
    assert!(err.to_string().contains("Multiple workspaces configured"));
    assert!(err.to_string().contains("--workspace"));
}

#[test]
fn resolve_selector_from_list_matches_unique_folder_name() {
    let ws1 = WorkspaceConfig::new_from_root(std::path::Path::new("/tmp/alpha"));
    let ws2 = WorkspaceConfig::new_from_root(std::path::Path::new("/tmp/bravo"));

    let resolved = WorkspacePolicy::resolve_selector_from_list("bravo", vec![ws1, ws2]).unwrap();
    assert_eq!(resolved.root_path, std::path::PathBuf::from("/tmp/bravo"));
}

#[test]
fn resolve_selector_from_list_errors_when_folder_name_is_ambiguous() {
    let ws1 = WorkspaceConfig::new_from_root(std::path::Path::new("/tmp/team-a/work"));
    let ws2 = WorkspaceConfig::new_from_root(std::path::Path::new("/tmp/team-b/work"));

    let err = WorkspacePolicy::resolve_selector_from_list("work", vec![ws1, ws2]).unwrap_err();
    assert!(err.to_string().contains("ambiguous"));
    assert!(err.to_string().contains("explicit path"));
}

#[test]
fn looks_like_path_identifies_path_like_selectors() {
    assert!(WorkspacePolicy::looks_like_path("~/repos"));
    assert!(WorkspacePolicy::looks_like_path("./repos"));
    assert!(WorkspacePolicy::looks_like_path("/tmp/repos"));
    assert!(!WorkspacePolicy::looks_like_path("work"));
}

#[test]
fn detect_from_cwd_returns_none_for_plain_tmp_dir() {
    let temp = tempfile::tempdir().unwrap();
    // No .git-same directory present, so detection should return None
    let result = WorkspacePolicy::detect_from_cwd(temp.path());
    assert!(result.is_none());
}

#[test]
fn detect_from_cwd_finds_workspace_root() {
    let temp = tempfile::tempdir().unwrap();
    let dot_dir = temp.path().join(".git-same");
    let config_path = dot_dir.join("config.toml");
    std::fs::create_dir_all(&dot_dir).unwrap();
    // Write a minimal workspace config
    let ws = WorkspaceConfig::new_from_root(temp.path());
    std::fs::write(&config_path, ws.to_toml().unwrap()).unwrap();

    let found = WorkspacePolicy::detect_from_cwd(temp.path());
    assert!(found.is_some());
}
