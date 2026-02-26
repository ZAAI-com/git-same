use super::*;
use crate::config::WorkspaceProvider;
use crate::types::ProviderKind;

#[test]
fn create_provider_supports_github() {
    let provider = WorkspaceProvider::default();
    let result = create_provider(&provider, "ghp_test_token").unwrap();
    assert_eq!(result.kind(), ProviderKind::GitHub);
}

#[test]
fn create_provider_returns_not_implemented_for_unsupported() {
    let unsupported = [
        (ProviderKind::GitHubEnterprise, "GitHub Enterprise"),
        (ProviderKind::GitLab, "GitLab"),
        (ProviderKind::GitLabSelfManaged, "GitLab Self-Managed"),
        (ProviderKind::Codeberg, "Codeberg"),
        (ProviderKind::Bitbucket, "Bitbucket"),
    ];

    for (kind, expected_name) in unsupported {
        let ws_provider = WorkspaceProvider {
            kind,
            api_url: None,
            prefer_ssh: true,
        };

        match create_provider(&ws_provider, "token") {
            Ok(_) => panic!("expected {} to be unsupported", expected_name),
            Err(err) => assert!(
                err.to_string().contains("coming soon"),
                "{} error should contain 'coming soon': {}",
                expected_name,
                err
            ),
        }
    }
}
