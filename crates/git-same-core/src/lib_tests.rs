use super::*;

#[test]
fn prelude_reexports_core_types() {
    use crate::prelude::*;

    let options = CloneOptions::new().with_depth(1).with_branch("main");
    assert_eq!(options.depth, 1);
    assert_eq!(options.branch.as_deref(), Some("main"));

    let provider = WorkspaceProvider::default();
    assert_eq!(provider.kind, ProviderKind::GitHub);

    let repo = Repo::test("rocket", "acme");
    let owned = OwnedRepo::new("acme", repo);
    assert_eq!(owned.full_name(), "acme/rocket");

    let progress = ProgressEvent::DiscoveryOrgsDiscovered { count: 1 };
    assert!(matches!(
        progress,
        ProgressEvent::DiscoveryOrgsDiscovered { count: 1 }
    ));

    let setup = SetupState::new("~/Git-Same/GitHub");
    assert_eq!(setup.step, SetupStep::Requirements);
}

#[test]
fn top_level_modules_are_accessible() {
    let _ = discovery::DiscoveryOrchestrator::new(
        config::FilterOptions::default(),
        "{org}/{repo}".to_string(),
    );
    let _ = output::Verbosity::Normal;
    let _ = operations::sync::SyncMode::Fetch;
    let _ = progress::ProgressEvent::DiscoveryPersonalReposStarted;
    let _ = setup::SetupStep::Requirements;
    let _ = types::ProviderKind::GitLab;
}
