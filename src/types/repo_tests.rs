use super::*;

#[test]
fn test_org_creation() {
    let org = Org::new("rust-lang", 1234);
    assert_eq!(org.login, "rust-lang");
    assert_eq!(org.id, 1234);
    assert!(org.description.is_none());
}

#[test]
fn test_repo_owner_extraction() {
    let repo = Repo::test("gisa", "user");
    assert_eq!(repo.owner(), "user");
}

#[test]
fn test_owned_repo() {
    let repo = Repo::test("gisa", "my-org");
    let owned = OwnedRepo::new("my-org", repo);
    assert_eq!(owned.owner, "my-org");
    assert_eq!(owned.name(), "gisa");
    assert_eq!(owned.full_name(), "my-org/gisa");
}

#[test]
fn test_action_plan_empty() {
    let plan = ActionPlan::new();
    assert!(plan.is_empty());
    assert_eq!(plan.total(), 0);
}

#[test]
fn test_action_plan_add_repos() {
    let mut plan = ActionPlan::new();

    let repo1 = OwnedRepo::new("org", Repo::test("repo1", "org"));
    let repo2 = OwnedRepo::new("org", Repo::test("repo2", "org"));
    let repo3 = OwnedRepo::new("org", Repo::test("repo3", "org"));

    plan.add_clone(repo1);
    plan.add_sync(repo2);
    plan.add_skipped(repo3, "already up to date");

    assert!(!plan.is_empty());
    assert_eq!(plan.to_clone.len(), 1);
    assert_eq!(plan.to_sync.len(), 1);
    assert_eq!(plan.skipped.len(), 1);
    assert_eq!(plan.total(), 3);
}

#[test]
fn test_op_result_methods() {
    let success = OpResult::Success;
    assert!(success.is_success());
    assert!(!success.is_failed());
    assert!(!success.is_skipped());
    assert!(success.error_message().is_none());

    let failed = OpResult::Failed("network error".to_string());
    assert!(!failed.is_success());
    assert!(failed.is_failed());
    assert_eq!(failed.error_message(), Some("network error"));

    let skipped = OpResult::Skipped("already exists".to_string());
    assert!(!skipped.is_success());
    assert!(skipped.is_skipped());
    assert_eq!(skipped.skip_reason(), Some("already exists"));
}

#[test]
fn test_op_summary() {
    let mut summary = OpSummary::new();
    assert_eq!(summary.total(), 0);
    assert!(!summary.has_failures());

    summary.record(&OpResult::Success);
    summary.record(&OpResult::Success);
    summary.record(&OpResult::Failed("error".to_string()));
    summary.record(&OpResult::Skipped("reason".to_string()));

    assert_eq!(summary.success, 2);
    assert_eq!(summary.failed, 1);
    assert_eq!(summary.skipped, 1);
    assert_eq!(summary.total(), 4);
    assert!(summary.has_failures());
}

#[test]
fn test_repo_serialization() {
    let repo = Repo::test("gisa", "user");
    let json = serde_json::to_string(&repo).unwrap();
    assert!(json.contains("\"name\":\"gisa\""));
    assert!(json.contains("\"full_name\":\"user/gisa\""));

    let deserialized: Repo = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.name, repo.name);
    assert_eq!(deserialized.full_name, repo.full_name);
}

#[test]
fn test_org_serialization() {
    let org = Org::new("rust-lang", 1234);
    let json = serde_json::to_string(&org).unwrap();
    assert!(json.contains("\"login\":\"rust-lang\""));

    let deserialized: Org = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized, org);
}
