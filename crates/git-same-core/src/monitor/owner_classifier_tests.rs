use super::*;
use crate::config::{WorkspaceConfig, WorkspaceProvider};
use crate::types::ProviderKind;
use std::path::Path;
use tempfile::TempDir;

/// Write a workspace `config.toml` directly so `WorkspaceStore::load` can read
/// it back, without touching the global registry or `HOME`.
fn write_workspace(root: &Path, provider: WorkspaceProvider, orgs: &[&str]) {
    let mut ws = WorkspaceConfig::new_from_root(root);
    ws.provider = provider;
    ws.orgs = orgs.iter().map(|s| s.to_string()).collect();
    let dot = root.join(".git-same");
    std::fs::create_dir_all(&dot).unwrap();
    std::fs::write(dot.join("config.toml"), ws.to_toml().unwrap()).unwrap();
}

fn ghe_provider(api_url: &str) -> WorkspaceProvider {
    WorkspaceProvider {
        kind: ProviderKind::GitHub,
        api_url: Some(api_url.to_string()),
        prefer_ssh: true,
    }
}

#[test]
fn groups_owner_names_by_provider_endpoint() {
    let dir = TempDir::new().unwrap();
    let dotcom_root = dir.path().join("dotcom");
    let ghe_root = dir.path().join("ghe");
    std::fs::create_dir_all(&dotcom_root).unwrap();
    std::fs::create_dir_all(&ghe_root).unwrap();

    write_workspace(
        &dotcom_root,
        WorkspaceProvider::default(),
        &["acme", "globex"],
    );
    write_workspace(
        &ghe_root,
        ghe_provider("https://github.example.com/api/v3"),
        &["internal-org"],
    );

    let config = Config {
        workspaces: vec![
            dotcom_root.to_string_lossy().into_owned(),
            ghe_root.to_string_lossy().into_owned(),
        ],
        ..Default::default()
    };

    let groups = collect_owner_names_by_provider(&config);
    assert_eq!(groups.len(), 2, "two distinct provider endpoints");

    let dotcom_url = WorkspaceProvider::default().effective_api_url();
    let dotcom = groups
        .iter()
        .find(|(p, _)| p.effective_api_url() == dotcom_url)
        .expect("github.com group present");
    assert_eq!(dotcom.1, vec!["acme".to_string(), "globex".to_string()]);

    let ghe = groups
        .iter()
        .find(|(p, _)| p.effective_api_url() == "https://github.example.com/api/v3")
        .expect("GHE group present");
    assert_eq!(ghe.1, vec!["internal-org".to_string()]);
}

#[test]
fn merges_owners_from_workspaces_on_same_endpoint() {
    let dir = TempDir::new().unwrap();
    let a = dir.path().join("a");
    let b = dir.path().join("b");
    std::fs::create_dir_all(&a).unwrap();
    std::fs::create_dir_all(&b).unwrap();
    write_workspace(&a, WorkspaceProvider::default(), &["acme"]);
    write_workspace(&b, WorkspaceProvider::default(), &["globex"]);

    let config = Config {
        workspaces: vec![
            a.to_string_lossy().into_owned(),
            b.to_string_lossy().into_owned(),
        ],
        ..Default::default()
    };

    let groups = collect_owner_names_by_provider(&config);
    assert_eq!(groups.len(), 1, "same endpoint collapses to one group");
    assert_eq!(groups[0].1, vec!["acme".to_string(), "globex".to_string()]);
}
