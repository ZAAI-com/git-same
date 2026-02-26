use super::*;

#[test]
fn test_default_provider_entry() {
    let entry = ProviderEntry::default();
    assert_eq!(entry.kind, ProviderKind::GitHub);
    assert_eq!(entry.auth, AuthMethod::GhCli);
    assert!(entry.prefer_ssh);
    assert!(entry.enabled);
}

#[test]
fn test_github_factory() {
    let entry = ProviderEntry::github();
    assert_eq!(entry.kind, ProviderKind::GitHub);
    assert_eq!(entry.display_name(), "GitHub");
}

#[test]
fn test_github_enterprise_factory() {
    let entry = ProviderEntry::github_enterprise(
        "https://github.company.com/api/v3",
        "COMPANY_GITHUB_TOKEN",
    );
    assert_eq!(entry.kind, ProviderKind::GitHubEnterprise);
    assert_eq!(entry.auth, AuthMethod::Env);
    assert_eq!(entry.token_env, Some("COMPANY_GITHUB_TOKEN".to_string()));
}

#[test]
fn test_effective_api_url_with_override() {
    let mut entry = ProviderEntry::github();
    entry.api_url = Some("https://custom-api.example.com".to_string());
    assert_eq!(entry.effective_api_url(), "https://custom-api.example.com");
}

#[test]
fn test_effective_api_url_default() {
    let entry = ProviderEntry::github();
    assert_eq!(entry.effective_api_url(), "https://api.github.com");
}

#[test]
fn test_validate_github_enterprise_without_url() {
    let entry = ProviderEntry {
        kind: ProviderKind::GitHubEnterprise,
        api_url: None,
        ..Default::default()
    };
    let result = entry.validate();
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("api_url"));
}

#[test]
fn test_validate_env_auth_without_token_env() {
    let entry = ProviderEntry {
        auth: AuthMethod::Env,
        token_env: None,
        ..Default::default()
    };
    let result = entry.validate();
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("token_env"));
}

#[test]
fn test_validate_token_auth_without_token() {
    let entry = ProviderEntry {
        auth: AuthMethod::Token,
        token: None,
        ..Default::default()
    };
    let result = entry.validate();
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("token"));
}

#[test]
fn test_validate_valid_config() {
    let entry = ProviderEntry::github();
    assert!(entry.validate().is_ok());

    let entry = ProviderEntry {
        auth: AuthMethod::Env,
        token_env: Some("MY_TOKEN".to_string()),
        ..Default::default()
    };
    assert!(entry.validate().is_ok());
}

#[test]
fn test_serde_roundtrip() {
    let entry = ProviderEntry {
        kind: ProviderKind::GitHub,
        name: Some("My GitHub".to_string()),
        auth: AuthMethod::Env,
        token_env: Some("MY_TOKEN".to_string()),
        prefer_ssh: false,
        ..Default::default()
    };

    let toml = toml::to_string(&entry).unwrap();
    let parsed: ProviderEntry = toml::from_str(&toml).unwrap();

    assert_eq!(parsed.kind, entry.kind);
    assert_eq!(parsed.name, entry.name);
    assert_eq!(parsed.auth, entry.auth);
    assert_eq!(parsed.token_env, entry.token_env);
    assert_eq!(parsed.prefer_ssh, entry.prefer_ssh);
}

#[test]
fn test_auth_method_serde() {
    assert_eq!(
        serde_json::to_string(&AuthMethod::GhCli).unwrap(),
        "\"gh-cli\""
    );
    assert_eq!(serde_json::to_string(&AuthMethod::Env).unwrap(), "\"env\"");
    assert_eq!(
        serde_json::to_string(&AuthMethod::Token).unwrap(),
        "\"token\""
    );
}
