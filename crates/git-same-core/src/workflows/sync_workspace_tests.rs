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
        workspace: WorkspaceConfig::new_from_root(std::path::Path::new("/tmp")),
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
    let workspace = WorkspaceConfig::new_from_root(std::path::Path::new("/tmp/team"));

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
    assert_eq!(
        request.workspace.root_path,
        std::path::PathBuf::from("/tmp/team")
    );
}

#[tokio::test]
async fn phases_return_none_without_touching_progress_when_idle() {
    let prepared = prepared_workspace(false, false);
    assert!(execute_prepared_clone(&prepared, Arc::new(NoProgress))
        .await
        .is_none());
    assert!(execute_prepared_fetch(&prepared, Arc::new(NoSyncProgress))
        .await
        .is_none());
}

#[test]
fn from_phases_keeps_summaries_and_results_of_both_phases() {
    use crate::operations::clone::CloneResult;
    use crate::types::OpResult;

    let mut clone_summary = OpSummary::new();
    clone_summary.record(&OpResult::Failed("boom".to_string()));
    let clone_results = vec![CloneResult {
        repo: sample_repo(),
        path: PathBuf::from("/tmp/acme/rocket"),
        result: OpResult::Failed("boom".to_string()),
    }];

    let outcome = SyncExecutionOutcome::from_phases(Some((clone_summary, clone_results)), None);
    assert_eq!(outcome.clone_summary.unwrap().failed, 1);
    assert_eq!(outcome.clone_results.len(), 1);
    assert!(outcome.sync_summary.is_none());
    assert!(outcome.sync_results.is_empty());
}

#[test]
fn skipped_at_planning_leaves_out_repos_about_to_be_cloned() {
    let mut prepared = prepared_workspace(true, false);
    let other = OwnedRepo::new("acme", Repo::test("dirty", "acme"));
    prepared.skipped_sync = vec![
        (sample_repo(), "not cloned locally".to_string()),
        (other.clone(), "uncommitted changes".to_string()),
    ];

    let skipped = prepared.skipped_at_planning();
    assert_eq!(skipped.len(), 1);
    assert_eq!(skipped[0].0.full_name(), other.full_name());
    assert_eq!(skipped[0].1, "uncommitted changes");
}
