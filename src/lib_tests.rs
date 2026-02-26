use super::*;

#[test]
fn prelude_reexports_core_types() {
    use crate::prelude::*;

    let options = CloneOptions::new().with_depth(1).with_branch("main");
    assert_eq!(options.depth, 1);
    assert_eq!(options.branch.as_deref(), Some("main"));

    let provider = ProviderEntry::github();
    assert_eq!(provider.kind, ProviderKind::GitHub);

    let repo = Repo::test("rocket", "acme");
    let owned = OwnedRepo::new("acme", repo);
    assert_eq!(owned.full_name(), "acme/rocket");
}

#[test]
fn top_level_modules_are_accessible() {
    let _ = output::Verbosity::Normal;
    let _ = operations::sync::SyncMode::Fetch;
    let _ = types::ProviderKind::GitLab;
}
