use super::*;

#[test]
fn test_new_from_root_workspace_config() {
    let ws = WorkspaceConfig::new_from_root(std::path::Path::new("/tmp/github"));
    assert_eq!(ws.root_path, std::path::PathBuf::from("/tmp/github"));
    assert_eq!(ws.provider.kind, ProviderKind::GitHub);
    assert!(ws.orgs.is_empty());
    assert!(ws.last_synced.is_none());
}

#[test]
fn test_workspace_provider_default() {
    let provider = WorkspaceProvider::default();
    assert_eq!(provider.kind, ProviderKind::GitHub);
    assert!(provider.prefer_ssh);
    assert!(provider.api_url.is_none());
}

#[test]
fn test_workspace_provider_effective_api_url() {
    let provider = WorkspaceProvider {
        kind: ProviderKind::GitHub,
        api_url: Some("https://custom-api.example.com".to_string()),
        prefer_ssh: true,
    };
    assert_eq!(
        provider.effective_api_url(),
        "https://custom-api.example.com"
    );
}

#[test]
fn test_workspace_provider_display_name() {
    let provider = WorkspaceProvider::default();
    assert_eq!(provider.display_name(), "GitHub");
}

#[test]
fn test_serde_roundtrip() {
    let ws = WorkspaceConfig {
        root_path: std::path::PathBuf::from("/tmp/repos"),
        provider: WorkspaceProvider {
            kind: ProviderKind::GitHub,
            api_url: None,
            prefer_ssh: true,
        },
        username: "testuser".to_string(),
        orgs: vec!["org1".to_string(), "org2".to_string()],
        include_repos: vec![],
        exclude_repos: vec!["org1/skip-this".to_string()],
        structure: Some("{org}/{repo}".to_string()),
        sync_mode: Some(SyncMode::Pull),
        clone_options: None,
        filters: FilterOptions {
            include_archived: false,
            include_forks: true,
            orgs: vec![],
            exclude_repos: vec![],
        },
        concurrency: Some(8),
        refresh_interval: None,
        last_synced: Some("2026-02-23T10:00:00Z".to_string()),
    };

    let toml_str = ws.to_toml().unwrap();
    let parsed = WorkspaceConfig::from_toml(&toml_str).unwrap();

    // root_path is skip — not written to TOML, so it's empty after parse
    assert_eq!(parsed.root_path, std::path::PathBuf::new());
    assert_eq!(parsed.username, ws.username);
    assert_eq!(parsed.orgs, ws.orgs);
    assert_eq!(parsed.exclude_repos, ws.exclude_repos);
    assert_eq!(parsed.structure, ws.structure);
    assert_eq!(parsed.sync_mode, ws.sync_mode);
    assert_eq!(parsed.concurrency, ws.concurrency);
    assert_eq!(parsed.last_synced, ws.last_synced);
    assert_eq!(parsed.provider.kind, ws.provider.kind);
    assert!(parsed.filters.include_forks);
}

#[test]
fn test_expanded_base_path_returns_root_path() {
    let ws = WorkspaceConfig::new_from_root(std::path::Path::new("/tmp/github"));
    let expanded = ws.expanded_base_path();
    assert_eq!(expanded, std::path::PathBuf::from("/tmp/github"));
}

#[test]
fn test_summary_with_orgs() {
    let ws = WorkspaceConfig {
        orgs: vec!["org1".to_string(), "org2".to_string()],
        last_synced: None,
        ..WorkspaceConfig::new_from_root(std::path::Path::new("/tmp/github"))
    };
    let summary = ws.summary();
    assert!(summary.contains("2 org(s)"));
    assert!(summary.contains("never synced"));
}

#[test]
fn test_display_label() {
    let ws = WorkspaceConfig::new_from_root(std::path::Path::new("/tmp/repos"));
    let label = ws.display_label();
    assert!(label.contains("GitHub"));
    assert!(label.contains("/tmp/repos") || label.contains("~/"));
}

#[test]
fn test_summary_all_orgs() {
    let ws = WorkspaceConfig::new_from_root(std::path::Path::new("/tmp/work"));
    let summary = ws.summary();
    assert!(summary.contains("all orgs"));
}

#[test]
fn test_optional_fields_not_serialized_when_none() {
    let ws = WorkspaceConfig::new_from_root(std::path::Path::new("/tmp/minimal"));
    let toml_str = ws.to_toml().unwrap();
    // root_path is skip_serializing — never written to TOML
    assert!(
        !toml_str.lines().any(|l| l.starts_with("root_path")),
        "TOML should not contain a 'root_path' key"
    );
    assert!(!toml_str.contains("structure"));
    assert!(!toml_str.contains("sync_mode"));
    assert!(!toml_str.contains("concurrency"));
    assert!(!toml_str.contains("last_synced"));
}

#[test]
fn test_tilde_collapse_path_replaces_home() {
    let home = std::env::var("HOME").unwrap_or_default();
    if home.is_empty() {
        return; // Skip if HOME not set
    }
    let path = std::path::Path::new(&home).join("repos");
    let collapsed = tilde_collapse_path(&path);
    assert!(collapsed.starts_with('~'));
}

#[test]
fn test_tilde_collapse_path_home_exactly() {
    let home = std::env::var("HOME").unwrap_or_default();
    if home.is_empty() {
        return;
    }
    let collapsed = tilde_collapse_path(std::path::Path::new(&home));
    assert_eq!(collapsed, "~");
}

#[test]
fn test_tilde_collapse_path_does_not_match_string_prefix_only() {
    let home = std::env::var("HOME").unwrap_or_default();
    if home.is_empty() {
        return;
    }
    let fake = std::path::PathBuf::from(format!("{}-other/repos", home));
    let collapsed = tilde_collapse_path(&fake);
    assert_eq!(collapsed, fake.to_string_lossy());
}
