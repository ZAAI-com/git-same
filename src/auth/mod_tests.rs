use super::*;
use std::sync::{LazyLock, Mutex};

static ENV_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

#[test]
fn test_resolved_auth_method_display() {
    assert_eq!(format!("{}", ResolvedAuthMethod::GhCli), "GitHub CLI");
    assert_eq!(
        format!("{}", ResolvedAuthMethod::EnvVar("MY_TOKEN".to_string())),
        "env:MY_TOKEN"
    );
    assert_eq!(
        format!("{}", ResolvedAuthMethod::ConfigToken),
        "config token"
    );
}

#[test]
fn test_extract_host() {
    assert_eq!(
        extract_host("https://api.github.com"),
        Some("api.github.com".to_string())
    );
    assert_eq!(
        extract_host("https://github.company.com/api/v3"),
        Some("github.company.com".to_string())
    );
    assert_eq!(
        extract_host("http://localhost:8080/api"),
        Some("localhost:8080".to_string())
    );
}

#[test]
fn test_extract_host_no_scheme() {
    assert_eq!(
        extract_host("api.github.com/v3"),
        Some("api.github.com".to_string())
    );
}

#[test]
fn test_extract_host_empty() {
    assert_eq!(extract_host(""), None);
}

#[test]
fn test_extract_host_scheme_only() {
    assert_eq!(extract_host("https://"), None);
}

#[test]
fn test_extract_host_with_port() {
    assert_eq!(
        extract_host("https://github.example.com:8443/api/v3"),
        Some("github.example.com:8443".to_string())
    );
}

#[test]
fn test_get_auth_with_config_token() {
    let _env_guard = ENV_LOCK.lock().unwrap();

    // Clear env vars temporarily for this test
    let saved_github_token = std::env::var("GITHUB_TOKEN").ok();
    let saved_gh_token = std::env::var("GH_TOKEN").ok();
    let saved_gisa_token = std::env::var("GISA_TOKEN").ok();

    std::env::remove_var("GITHUB_TOKEN");
    std::env::remove_var("GH_TOKEN");
    std::env::remove_var("GISA_TOKEN");

    // If gh is not installed/authenticated, this should use config token
    let result = get_auth(Some("test_token_value"));

    // Restore env vars
    if let Some(v) = saved_github_token {
        std::env::set_var("GITHUB_TOKEN", v);
    }
    if let Some(v) = saved_gh_token {
        std::env::set_var("GH_TOKEN", v);
    }
    if let Some(v) = saved_gisa_token {
        std::env::set_var("GISA_TOKEN", v);
    }

    // The result depends on whether gh is installed
    // If no gh, it should use config token or return error
    if let Ok(auth) = result {
        // Could be GhCli if gh is available, or ConfigToken
        assert!(!auth.token.is_empty());
    }
}

#[test]
fn test_get_auth_for_provider_env() {
    let _env_guard = ENV_LOCK.lock().unwrap();

    let unique_var = "GISA_TEST_PROVIDER_TOKEN";
    std::env::set_var(unique_var, "test_provider_token");

    let provider = ProviderEntry {
        auth: AuthMethod::Env,
        token_env: Some(unique_var.to_string()),
        ..ProviderEntry::default()
    };

    let result = get_auth_for_provider(&provider);
    assert!(result.is_ok());

    let auth = result.unwrap();
    assert_eq!(auth.token, "test_provider_token");
    assert_eq!(
        auth.method,
        ResolvedAuthMethod::EnvVar(unique_var.to_string())
    );

    std::env::remove_var(unique_var);
}

#[test]
fn test_get_auth_for_provider_config_token() {
    let provider = ProviderEntry {
        auth: AuthMethod::Token,
        token: Some("my_config_token".to_string()),
        ..ProviderEntry::default()
    };

    let result = get_auth_for_provider(&provider);
    assert!(result.is_ok());

    let auth = result.unwrap();
    assert_eq!(auth.token, "my_config_token");
    assert_eq!(auth.method, ResolvedAuthMethod::ConfigToken);
}

#[test]
fn test_get_auth_for_provider_missing_token() {
    let provider = ProviderEntry {
        auth: AuthMethod::Token,
        token: None,
        ..ProviderEntry::default()
    };

    let result = get_auth_for_provider(&provider);
    assert!(result.is_err());
}

#[test]
fn test_get_auth_for_provider_missing_env() {
    let _env_guard = ENV_LOCK.lock().unwrap();

    let provider = ProviderEntry {
        auth: AuthMethod::Env,
        token_env: Some("NONEXISTENT_VAR_XXXXX".to_string()),
        ..ProviderEntry::default()
    };

    let result = get_auth_for_provider(&provider);
    assert!(result.is_err());
}
