use super::*;

#[test]
fn name_from_path_uses_provider_prefix_and_normalizes() {
    let name = WorkspacePolicy::name_from_path(
        std::path::Path::new("~/Developer/My_Project"),
        ProviderKind::GitHubEnterprise,
    );
    assert_eq!(name, "ghe-my-project");

    let github = WorkspacePolicy::name_from_path(
        std::path::Path::new("~/repos/Personal"),
        ProviderKind::GitHub,
    );
    assert_eq!(github, "github-personal");
}

#[test]
fn resolve_from_list_errors_when_no_workspaces() {
    let err = WorkspacePolicy::resolve_from_list(Vec::new()).unwrap_err();
    assert!(err.to_string().contains("No workspaces configured"));
}

#[test]
fn resolve_from_list_returns_single_workspace() {
    let ws = WorkspaceConfig::new("solo", "/tmp/solo");
    let resolved = WorkspacePolicy::resolve_from_list(vec![ws.clone()]).unwrap();
    assert_eq!(resolved.name, "solo");
    assert_eq!(resolved.base_path, "/tmp/solo");
}

#[test]
fn resolve_from_list_errors_when_multiple_workspaces() {
    let ws1 = WorkspaceConfig::new("a", "/tmp/a");
    let ws2 = WorkspaceConfig::new("b", "/tmp/b");

    let err = WorkspacePolicy::resolve_from_list(vec![ws1, ws2]).unwrap_err();
    assert!(err.to_string().contains("Multiple workspaces configured"));
    assert!(err.to_string().contains("--workspace"));
}
