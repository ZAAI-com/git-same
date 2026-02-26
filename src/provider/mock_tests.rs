use super::*;

fn test_org(name: &str) -> Org {
    Org::new(name, 1)
}

#[tokio::test]
async fn test_mock_provider_username() {
    let provider = MockProvider::new().with_username("octocat");
    let username = provider.get_username().await.unwrap();
    assert_eq!(username, "octocat");
}

#[tokio::test]
async fn test_mock_provider_orgs() {
    let provider = MockProvider::new().with_orgs(vec![test_org("org1"), test_org("org2")]);

    let orgs = provider.get_organizations().await.unwrap();
    assert_eq!(orgs.len(), 2);
    assert_eq!(orgs[0].login, "org1");
    assert_eq!(orgs[1].login, "org2");
}

#[tokio::test]
async fn test_mock_provider_auth_failure() {
    let provider = MockProvider::new().with_auth_failure();

    let result = provider.validate_credentials().await;
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        ProviderError::Authentication(_)
    ));
}

#[tokio::test]
async fn test_mock_provider_orgs_failure() {
    let provider = MockProvider::new().with_orgs_failure();

    let result = provider.get_organizations().await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_mock_provider_call_logging() {
    let provider = MockProvider::new();

    provider.get_username().await.unwrap();
    provider.get_organizations().await.unwrap();
    provider.get_org_repos("test-org").await.unwrap();

    let calls = provider.get_calls();
    assert_eq!(calls.len(), 3);
    assert_eq!(calls[0], "get_username");
    assert_eq!(calls[1], "get_organizations");
    assert_eq!(calls[2], "get_org_repos:test-org");
}

#[tokio::test]
async fn test_mock_provider_discovery() {
    let provider = MockProvider::new()
        .with_username("testuser")
        .with_orgs(vec![test_org("my-org")])
        .with_org_repos("my-org", vec![Repo::test("repo1", "my-org")])
        .with_user_repos(vec![Repo::test("personal", "testuser")]);

    let options = DiscoveryOptions::new();
    let progress = NoProgress;

    let repos = provider.discover_repos(&options, &progress).await.unwrap();
    assert_eq!(repos.len(), 2);
}

#[tokio::test]
async fn test_mock_provider_discovery_with_filters() {
    let mut archived_repo = Repo::test("archived", "my-org");
    archived_repo.archived = true;

    let provider = MockProvider::new()
        .with_username("testuser")
        .with_orgs(vec![test_org("my-org")])
        .with_org_repos(
            "my-org",
            vec![Repo::test("active", "my-org"), archived_repo],
        );

    let options = DiscoveryOptions::new().with_archived(false);
    let progress = NoProgress;

    let repos = provider.discover_repos(&options, &progress).await.unwrap();
    assert_eq!(repos.len(), 1);
    assert_eq!(repos[0].repo.name, "active");
}

#[test]
fn test_clone_url_preference() {
    let provider = MockProvider::new();
    let repo = Repo::test("test", "org");

    let ssh_url = provider.get_clone_url(&repo, true);
    assert!(ssh_url.starts_with("git@"));

    let https_url = provider.get_clone_url(&repo, false);
    assert!(https_url.starts_with("https://"));
}
