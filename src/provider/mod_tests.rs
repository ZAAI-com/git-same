use super::*;
use crate::types::ProviderKind;

#[test]
fn create_provider_supports_github() {
    let github = ProviderEntry::github();
    let provider = create_provider(&github, "ghp_test_token").unwrap();
    assert_eq!(provider.kind(), ProviderKind::GitHub);
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
        let mut entry = ProviderEntry::github();
        entry.kind = kind;

        match create_provider(&entry, "token") {
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
