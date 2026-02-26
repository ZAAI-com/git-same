use super::*;

#[test]
fn test_default_is_github() {
    assert_eq!(ProviderKind::default(), ProviderKind::GitHub);
}

#[test]
fn test_display() {
    assert_eq!(format!("{}", ProviderKind::GitHub), "GitHub");
    assert_eq!(
        format!("{}", ProviderKind::GitHubEnterprise),
        "GitHub Enterprise"
    );
    assert_eq!(format!("{}", ProviderKind::GitLab), "GitLab");
    assert_eq!(
        format!("{}", ProviderKind::GitLabSelfManaged),
        "GitLab Self-Managed"
    );
    assert_eq!(format!("{}", ProviderKind::Codeberg), "Codeberg");
    assert_eq!(format!("{}", ProviderKind::Bitbucket), "Bitbucket");
}

#[test]
fn test_from_str() {
    assert_eq!(
        "github".parse::<ProviderKind>().unwrap(),
        ProviderKind::GitHub
    );
    assert_eq!("gh".parse::<ProviderKind>().unwrap(), ProviderKind::GitHub);
    assert_eq!(
        "GITHUB".parse::<ProviderKind>().unwrap(),
        ProviderKind::GitHub
    );

    assert_eq!(
        "github-enterprise".parse::<ProviderKind>().unwrap(),
        ProviderKind::GitHubEnterprise
    );
    assert_eq!(
        "ghe".parse::<ProviderKind>().unwrap(),
        ProviderKind::GitHubEnterprise
    );

    assert_eq!(
        "gitlab".parse::<ProviderKind>().unwrap(),
        ProviderKind::GitLab
    );
    assert_eq!("gl".parse::<ProviderKind>().unwrap(), ProviderKind::GitLab);

    assert_eq!(
        "gitlab-self-managed".parse::<ProviderKind>().unwrap(),
        ProviderKind::GitLabSelfManaged
    );
    assert_eq!(
        "glsm".parse::<ProviderKind>().unwrap(),
        ProviderKind::GitLabSelfManaged
    );

    assert_eq!(
        "codeberg".parse::<ProviderKind>().unwrap(),
        ProviderKind::Codeberg
    );
    assert_eq!(
        "cb".parse::<ProviderKind>().unwrap(),
        ProviderKind::Codeberg
    );

    assert_eq!(
        "bitbucket".parse::<ProviderKind>().unwrap(),
        ProviderKind::Bitbucket
    );
    assert_eq!(
        "bb".parse::<ProviderKind>().unwrap(),
        ProviderKind::Bitbucket
    );
}

#[test]
fn test_from_str_invalid() {
    let result = "invalid".parse::<ProviderKind>();
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Unknown provider"));
}

#[test]
fn test_default_api_urls() {
    assert_eq!(
        ProviderKind::GitHub.default_api_url(),
        "https://api.github.com"
    );
    assert_eq!(
        ProviderKind::GitLab.default_api_url(),
        "https://gitlab.com/api/v4"
    );
    assert_eq!(
        ProviderKind::Codeberg.default_api_url(),
        "https://codeberg.org/api/v1"
    );
    assert_eq!(
        ProviderKind::Bitbucket.default_api_url(),
        "https://api.bitbucket.org/2.0"
    );
    // Self-hosted providers have empty default (must be configured)
    assert_eq!(ProviderKind::GitHubEnterprise.default_api_url(), "");
    assert_eq!(ProviderKind::GitLabSelfManaged.default_api_url(), "");
}

#[test]
fn test_slug() {
    assert_eq!(ProviderKind::GitHub.slug(), "github");
    assert_eq!(ProviderKind::GitHubEnterprise.slug(), "github-enterprise");
    assert_eq!(ProviderKind::GitLab.slug(), "gitlab");
    assert_eq!(
        ProviderKind::GitLabSelfManaged.slug(),
        "gitlab-self-managed"
    );
    assert_eq!(ProviderKind::Codeberg.slug(), "codeberg");
    assert_eq!(ProviderKind::Bitbucket.slug(), "bitbucket");
}

#[test]
fn test_requires_custom_url() {
    assert!(!ProviderKind::GitHub.requires_custom_url());
    assert!(ProviderKind::GitHubEnterprise.requires_custom_url());
    assert!(!ProviderKind::GitLab.requires_custom_url());
    assert!(ProviderKind::GitLabSelfManaged.requires_custom_url());
    assert!(!ProviderKind::Codeberg.requires_custom_url());
    assert!(!ProviderKind::Bitbucket.requires_custom_url());
}

#[test]
fn test_serde_serialization() {
    let json = serde_json::to_string(&ProviderKind::GitHub).unwrap();
    assert_eq!(json, "\"github\"");

    let json = serde_json::to_string(&ProviderKind::GitHubEnterprise).unwrap();
    assert_eq!(json, "\"github-enterprise\"");

    let json = serde_json::to_string(&ProviderKind::GitLabSelfManaged).unwrap();
    assert_eq!(json, "\"gitlab-self-managed\"");

    let json = serde_json::to_string(&ProviderKind::Codeberg).unwrap();
    assert_eq!(json, "\"codeberg\"");
}

#[test]
fn test_serde_deserialization() {
    let kind: ProviderKind = serde_json::from_str("\"github\"").unwrap();
    assert_eq!(kind, ProviderKind::GitHub);

    let kind: ProviderKind = serde_json::from_str("\"gitlab\"").unwrap();
    assert_eq!(kind, ProviderKind::GitLab);

    let kind: ProviderKind = serde_json::from_str("\"codeberg\"").unwrap();
    assert_eq!(kind, ProviderKind::Codeberg);
}

#[test]
fn test_all_providers() {
    let all = ProviderKind::all();
    assert_eq!(all.len(), 6);
    assert!(all.contains(&ProviderKind::GitHub));
    assert!(all.contains(&ProviderKind::GitHubEnterprise));
    assert!(all.contains(&ProviderKind::GitLab));
    assert!(all.contains(&ProviderKind::GitLabSelfManaged));
    assert!(all.contains(&ProviderKind::Codeberg));
    assert!(all.contains(&ProviderKind::Bitbucket));
}

#[test]
fn test_equality_and_hash() {
    use std::collections::HashSet;

    let mut set = HashSet::new();
    set.insert(ProviderKind::GitHub);
    set.insert(ProviderKind::GitHub); // Duplicate

    assert_eq!(set.len(), 1);
    assert!(set.contains(&ProviderKind::GitHub));
}
