use super::*;
use crate::types::ProviderKind;

#[test]
fn create_provider_supports_github_and_ghe() {
    let github = ProviderEntry::github();
    let provider = create_provider(&github, "ghp_test_token").unwrap();
    assert_eq!(provider.kind(), ProviderKind::GitHub);

    let ghe = ProviderEntry::github_enterprise("https://ghe.example/api/v3", "GHE_TOKEN");
    let provider = create_provider(&ghe, "ghe_test_token").unwrap();
    assert_eq!(provider.kind(), ProviderKind::GitHubEnterprise);
}

#[test]
fn create_provider_returns_not_implemented_for_gitlab_and_bitbucket() {
    let mut gitlab = ProviderEntry::github();
    gitlab.kind = ProviderKind::GitLab;

    match create_provider(&gitlab, "token") {
        Ok(_) => panic!("expected GitLab to be unsupported"),
        Err(err) => assert!(err.to_string().contains("GitLab support coming soon")),
    }

    let mut bitbucket = ProviderEntry::github();
    bitbucket.kind = ProviderKind::Bitbucket;

    match create_provider(&bitbucket, "token") {
        Ok(_) => panic!("expected Bitbucket to be unsupported"),
        Err(err) => assert!(err.to_string().contains("Bitbucket support coming soon")),
    }
}
