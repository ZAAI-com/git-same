use super::*;

#[test]
fn test_name_from_path_simple() {
    let name =
        WorkspaceManager::name_from_path(Path::new("/home/user/github"), ProviderKind::GitHub);
    assert_eq!(name, "github-github");
}
