use super::*;
use std::io::Write;
use tempfile::NamedTempFile;

#[test]
fn test_default_config() {
    let config = Config::default();
    assert_eq!(config.concurrency, 8);
    assert_eq!(config.sync_mode, SyncMode::Fetch);
    assert!(!config.filters.include_archived);
    assert!(!config.filters.include_forks);
    assert_eq!(config.providers.len(), 1);
}

#[test]
fn test_load_minimal_config() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, "concurrency = 2").unwrap();

    let config = Config::load_from(file.path()).unwrap();
    assert_eq!(config.concurrency, 2);
}

#[test]
fn test_load_full_config() {
    let content = r#"
structure = "{provider}/{org}/{repo}"
concurrency = 8
sync_mode = "pull"

[clone]
depth = 1
recurse_submodules = true

[filters]
include_archived = true
include_forks = true
orgs = ["my-org"]
exclude_repos = ["my-org/skip-this"]

[[providers]]
kind = "github"
auth = "gh-cli"
"#;

    let config = Config::parse(content).unwrap();
    assert_eq!(config.structure, "{provider}/{org}/{repo}");
    assert_eq!(config.concurrency, 8);
    assert_eq!(config.sync_mode, SyncMode::Pull);
    assert_eq!(config.clone.depth, 1);
    assert!(config.clone.recurse_submodules);
    assert!(config.filters.include_archived);
    assert!(config.filters.include_forks);
    assert_eq!(config.filters.orgs, vec!["my-org"]);
    assert_eq!(config.filters.exclude_repos, vec!["my-org/skip-this"]);
}

#[test]
fn test_load_multi_provider_config() {
    let content = r#"
[[providers]]
kind = "github"
auth = "gh-cli"

[[providers]]
kind = "github-enterprise"
name = "Work"
api_url = "https://github.work.com/api/v3"
auth = "env"
token_env = "WORK_TOKEN"
"#;

    let config = Config::parse(content).unwrap();
    assert_eq!(config.providers.len(), 2);
    assert_eq!(config.providers[0].kind, crate::types::ProviderKind::GitHub);
    assert_eq!(
        config.providers[1].kind,
        crate::types::ProviderKind::GitHubEnterprise
    );
    assert_eq!(config.providers[1].name, Some("Work".to_string()));
}

#[test]
fn test_missing_file_returns_defaults() {
    let config = Config::load_from(Path::new("/nonexistent/config.toml")).unwrap();
    assert_eq!(config.concurrency, 8);
}

#[test]
fn test_validation_rejects_zero_concurrency() {
    let config = Config {
        concurrency: 0,
        ..Config::default()
    };
    let result = config.validate();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("concurrency"));
}

#[test]
fn test_validation_rejects_high_concurrency() {
    let config = Config {
        concurrency: 100,
        ..Config::default()
    };
    let result = config.validate();
    assert!(result.is_err());
}

#[test]
fn test_validation_rejects_empty_providers() {
    let config = Config {
        providers: vec![],
        ..Config::default()
    };
    let result = config.validate();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("provider"));
}

#[test]
fn test_sync_mode_from_str() {
    assert_eq!("fetch".parse::<SyncMode>().unwrap(), SyncMode::Fetch);
    assert_eq!("pull".parse::<SyncMode>().unwrap(), SyncMode::Pull);
    assert_eq!("FETCH".parse::<SyncMode>().unwrap(), SyncMode::Fetch);
    assert!("invalid".parse::<SyncMode>().is_err());
}

#[test]
fn test_default_toml_is_valid() {
    let toml = Config::default_toml();
    let result = Config::parse(&toml);
    assert!(result.is_ok(), "Default TOML should be valid: {:?}", result);
}

#[test]
fn test_enabled_providers_filter() {
    let config = Config {
        providers: vec![
            ProviderEntry {
                enabled: true,
                ..ProviderEntry::github()
            },
            ProviderEntry {
                enabled: false,
                ..ProviderEntry::github()
            },
            ProviderEntry {
                enabled: true,
                ..ProviderEntry::github()
            },
        ],
        ..Config::default()
    };

    let enabled: Vec<_> = config.enabled_providers().collect();
    assert_eq!(enabled.len(), 2);
}

#[test]
fn test_default_config_has_no_default_workspace() {
    let config = Config::default();
    assert!(config.default_workspace.is_none());
}

#[test]
fn test_parse_config_with_default_workspace() {
    let content = r#"
default_workspace = "my-ws"

[[providers]]
kind = "github"
auth = "gh-cli"
"#;
    let config = Config::parse(content).unwrap();
    assert_eq!(config.default_workspace, Some("my-ws".to_string()));
}

#[test]
fn test_parse_config_without_default_workspace() {
    let content = r#"
[[providers]]
kind = "github"
auth = "gh-cli"
"#;
    let config = Config::parse(content).unwrap();
    assert!(config.default_workspace.is_none());
}

#[test]
fn test_save_default_workspace_to_set() {
    let temp = tempfile::TempDir::new().unwrap();
    let path = temp.path().join("config.toml");
    std::fs::write(&path, Config::default_toml()).unwrap();

    Config::save_default_workspace_to(&path, Some("my-ws")).unwrap();

    let content = std::fs::read_to_string(&path).unwrap();
    assert!(content.contains("default_workspace = \"my-ws\""));
    // Original content preserved
    assert!(content.contains("concurrency"));
    // Still valid TOML
    let config = Config::parse(&content).unwrap();
    assert_eq!(config.default_workspace, Some("my-ws".to_string()));
}

#[test]
fn test_save_default_workspace_to_clear() {
    let temp = tempfile::TempDir::new().unwrap();
    let path = temp.path().join("config.toml");
    std::fs::write(&path, Config::default_toml()).unwrap();

    // Set then clear
    Config::save_default_workspace_to(&path, Some("my-ws")).unwrap();
    Config::save_default_workspace_to(&path, None).unwrap();

    let content = std::fs::read_to_string(&path).unwrap();
    assert!(!content.contains("default_workspace"));
    // Still valid TOML
    let config = Config::parse(&content).unwrap();
    assert!(config.default_workspace.is_none());
}

#[test]
fn test_save_default_workspace_to_replace() {
    let temp = tempfile::TempDir::new().unwrap();
    let path = temp.path().join("config.toml");
    std::fs::write(&path, Config::default_toml()).unwrap();

    Config::save_default_workspace_to(&path, Some("ws1")).unwrap();
    Config::save_default_workspace_to(&path, Some("ws2")).unwrap();

    let content = std::fs::read_to_string(&path).unwrap();
    assert!(content.contains("default_workspace = \"ws2\""));
    assert!(!content.contains("ws1"));
    let config = Config::parse(&content).unwrap();
    assert_eq!(config.default_workspace, Some("ws2".to_string()));
}

#[test]
fn test_save_default_workspace_to_replace_without_sync_mode() {
    let temp = tempfile::TempDir::new().unwrap();
    let path = temp.path().join("config.toml");
    let content = r#"
structure = "{org}/{repo}"
concurrency = 8
default_workspace = "ws-old"

[[providers]]
kind = "github"
auth = "gh-cli"
"#;
    std::fs::write(&path, content).unwrap();

    Config::save_default_workspace_to(&path, Some("ws-new")).unwrap();

    let updated = std::fs::read_to_string(&path).unwrap();
    assert!(updated.contains("default_workspace = \"ws-new\""));
    assert!(!updated.contains("ws-old"));
    let config = Config::parse(&updated).unwrap();
    assert_eq!(config.default_workspace.as_deref(), Some("ws-new"));
}

#[test]
fn test_save_default_workspace_to_nonexistent_file() {
    let result =
        Config::save_default_workspace_to(Path::new("/nonexistent/config.toml"), Some("ws"));
    assert!(result.is_err());
}
