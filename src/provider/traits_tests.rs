use super::*;

#[test]
fn test_credentials_builder() {
    let creds = Credentials::new("token123", "https://api.github.com").with_username("testuser");

    assert_eq!(creds.token, "token123");
    assert_eq!(creds.api_base_url, "https://api.github.com");
    assert_eq!(creds.username, Some("testuser".to_string()));
}

#[test]
fn test_rate_limit_exhausted() {
    let info = RateLimitInfo {
        limit: 5000,
        remaining: 0,
        reset_at: None,
    };
    assert!(info.is_exhausted());

    let info = RateLimitInfo {
        limit: 5000,
        remaining: 100,
        reset_at: None,
    };
    assert!(!info.is_exhausted());
}

#[test]
fn test_discovery_options_builder() {
    let options = DiscoveryOptions::new()
        .with_archived(true)
        .with_forks(true)
        .with_orgs(vec!["org1".to_string(), "org2".to_string()])
        .with_exclusions(vec!["org1/skip".to_string()]);

    assert!(options.include_archived);
    assert!(options.include_forks);
    assert_eq!(options.org_filter.len(), 2);
    assert_eq!(options.exclude_repos.len(), 1);
}

#[test]
fn test_should_include_repo() {
    let options = DiscoveryOptions::new();

    // Non-archived, non-fork repo should be included
    let repo = Repo::test("repo", "org");
    assert!(options.should_include(&repo));
}

#[test]
fn test_should_exclude_archived() {
    let options = DiscoveryOptions::new().with_archived(false);

    let mut repo = Repo::test("repo", "org");
    repo.archived = true;
    assert!(!options.should_include(&repo));

    let options = DiscoveryOptions::new().with_archived(true);
    assert!(options.should_include(&repo));
}

#[test]
fn test_should_exclude_forks() {
    let options = DiscoveryOptions::new().with_forks(false);

    let mut repo = Repo::test("repo", "org");
    repo.fork = true;
    assert!(!options.should_include(&repo));

    let options = DiscoveryOptions::new().with_forks(true);
    assert!(options.should_include(&repo));
}

#[test]
fn test_should_exclude_by_name() {
    let options = DiscoveryOptions::new().with_exclusions(vec!["org/excluded-repo".to_string()]);

    let mut repo = Repo::test("excluded-repo", "org");
    repo.full_name = "org/excluded-repo".to_string();
    assert!(!options.should_include(&repo));

    let mut repo = Repo::test("included-repo", "org");
    repo.full_name = "org/included-repo".to_string();
    assert!(options.should_include(&repo));
}

#[test]
fn test_should_include_org_empty_filter() {
    let options = DiscoveryOptions::new();
    assert!(options.should_include_org("any-org"));
}

#[test]
fn test_should_include_org_with_filter() {
    let options = DiscoveryOptions::new().with_orgs(vec!["allowed-org".to_string()]);

    assert!(options.should_include_org("allowed-org"));
    assert!(!options.should_include_org("other-org"));
}

#[test]
fn test_no_progress_compiles() {
    let progress = NoProgress;
    progress.on_orgs_discovered(5);
    progress.on_org_started("test");
    progress.on_org_complete("test", 10);
    progress.on_personal_repos_started();
    progress.on_personal_repos_complete(3);
    progress.on_error("test error");
}
