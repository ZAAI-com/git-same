use super::*;

// Provider configuration is now handled by WorkspaceProvider in workspace.rs.
// These tests verify the WorkspaceProvider API used throughout the codebase.

use crate::config::WorkspaceProvider;
use crate::types::ProviderKind;

#[test]
fn test_default_workspace_provider() {
    let provider = WorkspaceProvider::default();
    assert_eq!(provider.kind, ProviderKind::GitHub);
    assert!(provider.prefer_ssh);
    assert!(provider.api_url.is_none());
}

#[test]
fn test_workspace_provider_effective_api_url_default() {
    let provider = WorkspaceProvider::default();
    assert_eq!(provider.effective_api_url(), "https://api.github.com");
}

#[test]
fn test_workspace_provider_effective_api_url_override() {
    let provider = WorkspaceProvider {
        kind: ProviderKind::GitHub,
        api_url: Some("https://github.example.com/api/v3".to_string()),
        prefer_ssh: true,
    };
    assert_eq!(
        provider.effective_api_url(),
        "https://github.example.com/api/v3"
    );
}

#[test]
fn test_workspace_provider_display_name() {
    let provider = WorkspaceProvider::default();
    assert_eq!(provider.display_name(), "GitHub");
}

#[test]
fn test_workspace_provider_serde_roundtrip() {
    let provider = WorkspaceProvider {
        kind: ProviderKind::GitHub,
        api_url: None,
        prefer_ssh: false,
    };

    let toml = toml::to_string(&provider).unwrap();
    let parsed: WorkspaceProvider = toml::from_str(&toml).unwrap();

    assert_eq!(parsed.kind, provider.kind);
    assert_eq!(parsed.api_url, provider.api_url);
    assert_eq!(parsed.prefer_ssh, provider.prefer_ssh);
}
