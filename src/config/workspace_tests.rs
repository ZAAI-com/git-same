use super::*;

#[test]
fn test_new_workspace_config() {
    let ws = WorkspaceConfig::new("github", "~/github");
    assert_eq!(ws.name, "github");
    assert_eq!(ws.base_path, "~/github");
    assert_eq!(ws.provider.kind, ProviderKind::GitHub);
    assert!(ws.orgs.is_empty());
    assert!(ws.last_synced.is_none());
}

#[test]
fn test_workspace_provider_default() {
    let provider = WorkspaceProvider::default();
    assert_eq!(provider.kind, ProviderKind::GitHub);
    assert_eq!(provider.auth, AuthMethod::GhCli);
    assert!(provider.prefer_ssh);
    assert!(provider.api_url.is_none());
}

#[test]
fn test_workspace_provider_to_provider_entry() {
    let provider = WorkspaceProvider {
        kind: ProviderKind::GitHub,
        auth: AuthMethod::GhCli,
        api_url: None,
        prefer_ssh: false,
    };
    let entry = provider.to_provider_entry();
    assert_eq!(entry.kind, ProviderKind::GitHub);
    assert_eq!(entry.auth, AuthMethod::GhCli);
    assert!(entry.api_url.is_none());
    assert!(!entry.prefer_ssh);
    assert!(entry.enabled);
}

#[test]
fn test_serde_roundtrip() {
    let ws = WorkspaceConfig {
        name: "my-workspace".to_string(),
        base_path: "~/code/repos".to_string(),
        provider: WorkspaceProvider {
            kind: ProviderKind::GitHub,
            auth: AuthMethod::GhCli,
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

    // name is skip_serializing — it's derived from the folder, not the TOML
    assert!(parsed.name.is_empty());
    assert_eq!(parsed.base_path, ws.base_path);
    assert_eq!(parsed.username, ws.username);
    assert_eq!(parsed.orgs, ws.orgs);
    assert_eq!(parsed.exclude_repos, ws.exclude_repos);
    assert_eq!(parsed.structure, ws.structure);
    assert_eq!(parsed.sync_mode, ws.sync_mode);
    assert_eq!(parsed.concurrency, ws.concurrency);
    assert_eq!(parsed.last_synced, ws.last_synced);
    assert_eq!(parsed.provider.kind, ws.provider.kind);
    assert_eq!(parsed.provider.auth, ws.provider.auth);
    assert!(parsed.filters.include_forks);
}

#[test]
fn test_expanded_base_path() {
    let ws = WorkspaceConfig::new("test", "~/github");
    let expanded = ws.expanded_base_path();
    assert!(!expanded.to_string_lossy().contains('~'));
}

#[test]
fn test_summary() {
    let ws = WorkspaceConfig {
        orgs: vec!["org1".to_string(), "org2".to_string()],
        last_synced: None,
        ..WorkspaceConfig::new("github", "~/github")
    };
    let summary = ws.summary();
    assert!(summary.contains("github"));
    assert!(summary.contains("2 org(s)"));
    assert!(summary.contains("never synced"));
}

#[test]
fn test_display_label() {
    let ws = WorkspaceConfig::new("github-repos", "~/repos");
    assert_eq!(ws.display_label(), "~/repos (GitHub)");
}

#[test]
fn test_summary_all_orgs() {
    let ws = WorkspaceConfig::new("work", "~/work");
    let summary = ws.summary();
    assert!(summary.contains("all orgs"));
}

#[test]
fn test_optional_fields_not_serialized_when_none() {
    let ws = WorkspaceConfig::new("minimal", "~/minimal");
    let toml_str = ws.to_toml().unwrap();
    // name is derived from folder, never written to TOML as its own key
    assert!(
        !toml_str.lines().any(|l| l.starts_with("name ")),
        "TOML should not contain a 'name' key"
    );
    assert!(!toml_str.contains("structure"));
    assert!(!toml_str.contains("sync_mode"));
    assert!(!toml_str.contains("concurrency"));
    assert!(!toml_str.contains("last_synced"));
}
