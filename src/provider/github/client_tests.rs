use super::*;

fn test_credentials() -> Credentials {
    Credentials::new("test-token", GITHUB_API_URL)
}

#[test]
fn test_provider_creation() {
    let result = GitHubProvider::new(test_credentials(), "Test GitHub");
    assert!(result.is_ok());

    let provider = result.unwrap();
    assert_eq!(provider.kind(), ProviderKind::GitHub);
    assert_eq!(provider.display_name(), "Test GitHub");
}

#[test]
fn test_is_github_com() {
    let provider = GitHubProvider::new(test_credentials(), "GitHub").unwrap();
    assert!(provider.is_github_com());

    let enterprise_creds = Credentials::new("token", "https://github.company.com/api/v3");
    let provider = GitHubProvider::new(enterprise_creds, "GHE").unwrap();
    assert!(!provider.is_github_com());
}

#[test]
fn test_api_url_construction() {
    let provider = GitHubProvider::new(test_credentials(), "GitHub").unwrap();
    assert_eq!(provider.api_url("/user"), "https://api.github.com/user");
    assert_eq!(
        provider.api_url("/orgs/test/repos"),
        "https://api.github.com/orgs/test/repos"
    );
}

#[test]
fn test_kind_detection() {
    let github_creds = Credentials::new("token", GITHUB_API_URL);
    let provider = GitHubProvider::new(github_creds, "GitHub").unwrap();
    assert_eq!(provider.kind(), ProviderKind::GitHub);

    let ghe_creds = Credentials::new("token", "https://github.company.com/api/v3");
    let provider = GitHubProvider::new(ghe_creds, "GHE").unwrap();
    assert_eq!(provider.kind(), ProviderKind::GitHubEnterprise);
}

// Integration tests that require a real GitHub token
// These are ignored by default
#[tokio::test]
#[ignore]
async fn test_get_username_real() {
    let token = std::env::var("GITHUB_TOKEN").expect("GITHUB_TOKEN not set");
    let credentials = Credentials::new(token, GITHUB_API_URL);
    let provider = GitHubProvider::new(credentials, "GitHub").unwrap();

    let username = provider.get_username().await.unwrap();
    assert!(!username.is_empty());
}

#[tokio::test]
#[ignore]
async fn test_get_rate_limit_real() {
    let token = std::env::var("GITHUB_TOKEN").expect("GITHUB_TOKEN not set");
    let credentials = Credentials::new(token, GITHUB_API_URL);
    let provider = GitHubProvider::new(credentials, "GitHub").unwrap();

    let rate_limit = provider.get_rate_limit().await.unwrap();
    assert!(rate_limit.limit > 0);
}
