use super::*;
use crate::auth::{AuthResult, ResolvedAuthMethod};
use crate::config::{Config, WorkspaceConfig};
use crate::git::CloneOptions;
use crate::operations::clone::NoProgress;
use crate::operations::sync::{LocalRepo, NoSyncProgress, SyncMode};
use crate::types::{ActionPlan, OwnedRepo, Repo};
use std::path::PathBuf;
use std::sync::Arc;

fn sample_repo() -> OwnedRepo {
    OwnedRepo::new("acme", Repo::test("rocket", "acme"))
}

fn prepared_workspace(with_clone: bool, with_sync: bool) -> PreparedSyncWorkspace {
    let repo = sample_repo();
    let mut plan = ActionPlan::new();
    if with_clone {
        plan.add_clone(repo.clone());
    }

    let to_sync = if with_sync {
        vec![LocalRepo::new(repo.clone(), "/tmp/acme/rocket")]
    } else {
        Vec::new()
    };

    PreparedSyncWorkspace {
        workspace: WorkspaceConfig::new("ws", "/tmp"),
        auth: AuthResult {
            token: "token".to_string(),
            method: ResolvedAuthMethod::GhCli,
            username: Some("octocat".to_string()),
        },
        repos: vec![repo],
        used_cache: false,
        cache_age_secs: None,
        base_path: PathBuf::from("/tmp"),
        structure: "{org}/{repo}".to_string(),
        provider_name: "github".to_string(),
        provider_prefer_ssh: true,
        skip_uncommitted: true,
        sync_mode: SyncMode::Fetch,
        requested_concurrency: 4,
        effective_concurrency: 4,
        plan,
        to_sync,
        skipped_sync: Vec::new(),
        clone_options: CloneOptions::default(),
    }
}

#[tokio::test]
async fn execute_prepared_sync_dry_run_short_circuits() {
    let prepared = prepared_workspace(true, true);

    let outcome = execute_prepared_sync(
        &prepared,
        true,
        Arc::new(NoProgress),
        Arc::new(NoSyncProgress),
    )
    .await;

    assert!(outcome.clone_summary.is_none());
    assert!(outcome.sync_summary.is_none());
    assert!(outcome.sync_results.is_empty());
}

#[tokio::test]
async fn execute_prepared_sync_with_no_work_returns_empty_outcome() {
    let prepared = prepared_workspace(false, false);

    let outcome = execute_prepared_sync(
        &prepared,
        false,
        Arc::new(NoProgress),
        Arc::new(NoSyncProgress),
    )
    .await;

    assert!(outcome.clone_summary.is_none());
    assert!(outcome.sync_summary.is_none());
    assert!(outcome.sync_results.is_empty());
}

#[test]
fn sync_workspace_request_holds_expected_values() {
    let config = Config::default();
    let workspace = WorkspaceConfig::new("team", "/tmp/team");

    let request = SyncWorkspaceRequest {
        config: &config,
        workspace: &workspace,
        refresh: true,
        skip_uncommitted: false,
        pull: true,
        concurrency_override: Some(7),
        create_base_path: true,
    };

    assert!(request.refresh);
    assert!(request.pull);
    assert!(!request.skip_uncommitted);
    assert_eq!(request.concurrency_override, Some(7));
    assert!(request.create_base_path);
    assert_eq!(request.workspace.name, "team");
}
