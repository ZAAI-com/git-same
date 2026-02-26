use super::*;
use crate::config::Config;
use crate::git::{FetchResult, PullResult};
use crate::operations::clone::CloneProgress;
use crate::operations::sync::SyncProgress;
use crate::provider::DiscoveryProgress;
use crate::tui::event::{AppEvent, BackendMessage};
use crate::types::{OwnedRepo, Repo};
use tokio::sync::mpsc::error::TryRecvError;
use tokio::sync::mpsc::unbounded_channel;
use tokio::time::{timeout, Duration};

fn sample_repo() -> OwnedRepo {
    OwnedRepo::new("acme", Repo::test("rocket", "acme"))
}

fn expect_backend_event(event: AppEvent) -> BackendMessage {
    match event {
        AppEvent::Backend(msg) => msg,
        _ => panic!("expected backend event"),
    }
}

#[test]
fn discovery_progress_emits_expected_messages() {
    let (tx, mut rx) = unbounded_channel();
    let progress = TuiDiscoveryProgress { tx };

    progress.on_orgs_discovered(2);
    progress.on_org_started("acme");
    progress.on_org_complete("acme", 3);
    progress.on_error("boom");

    match expect_backend_event(rx.try_recv().expect("org count event")) {
        BackendMessage::OrgsDiscovered(count) => assert_eq!(count, 2),
        _ => panic!("expected OrgsDiscovered"),
    }

    match expect_backend_event(rx.try_recv().expect("org started event")) {
        BackendMessage::OrgStarted(org) => assert_eq!(org, "acme"),
        _ => panic!("expected OrgStarted"),
    }

    match expect_backend_event(rx.try_recv().expect("org complete event")) {
        BackendMessage::OrgComplete(org, count) => {
            assert_eq!(org, "acme");
            assert_eq!(count, 3);
        }
        _ => panic!("expected OrgComplete"),
    }

    match expect_backend_event(rx.try_recv().expect("error event")) {
        BackendMessage::DiscoveryError(msg) => assert_eq!(msg, "boom"),
        _ => panic!("expected DiscoveryError"),
    }

    progress.on_personal_repos_started();
    progress.on_personal_repos_complete(1);
    assert!(matches!(rx.try_recv(), Err(TryRecvError::Empty)));
}

#[test]
fn clone_progress_emits_started_complete_error_and_skip() {
    let (tx, mut rx) = unbounded_channel();
    let progress = TuiCloneProgress { tx };
    let repo = sample_repo();

    progress.on_start(&repo, 1, 4);
    progress.on_complete(&repo, 1, 4);
    progress.on_error(&repo, "clone failed", 2, 4);
    progress.on_skip(&repo, "already exists", 3, 4);

    match expect_backend_event(rx.try_recv().expect("started event")) {
        BackendMessage::RepoStarted { repo_name } => assert_eq!(repo_name, repo.full_name()),
        _ => panic!("expected RepoStarted"),
    }

    match expect_backend_event(rx.try_recv().expect("complete event")) {
        BackendMessage::RepoProgress {
            repo_name,
            success,
            skipped,
            is_clone,
            had_updates,
            skip_reason,
            ..
        } => {
            assert_eq!(repo_name, repo.full_name());
            assert!(success);
            assert!(!skipped);
            assert!(is_clone);
            assert!(had_updates);
            assert!(skip_reason.is_none());
        }
        _ => panic!("expected RepoProgress (complete)"),
    }

    match expect_backend_event(rx.try_recv().expect("error event")) {
        BackendMessage::RepoProgress {
            success,
            skipped,
            message,
            is_clone,
            ..
        } => {
            assert!(!success);
            assert!(!skipped);
            assert_eq!(message, "clone failed");
            assert!(is_clone);
        }
        _ => panic!("expected RepoProgress (error)"),
    }

    match expect_backend_event(rx.try_recv().expect("skip event")) {
        BackendMessage::RepoProgress {
            success,
            skipped,
            message,
            is_clone,
            skip_reason,
            ..
        } => {
            assert!(success);
            assert!(skipped);
            assert_eq!(message, "skipped: already exists");
            assert!(is_clone);
            assert_eq!(skip_reason.as_deref(), Some("already exists"));
        }
        _ => panic!("expected RepoProgress (skip)"),
    }
}

#[test]
fn sync_progress_emits_fetch_pull_error_and_skip() {
    let (tx, mut rx) = unbounded_channel();
    let progress = TuiSyncProgress { tx };
    let repo = sample_repo();

    progress.on_start(&repo, std::path::Path::new("/tmp"), 1, 3);

    let fetch = FetchResult {
        updated: true,
        new_commits: Some(5),
    };
    progress.on_fetch_complete(&repo, &fetch, 1, 3);

    let pull = PullResult {
        success: true,
        updated: true,
        fast_forward: true,
        error: None,
    };
    progress.on_pull_complete(&repo, &pull, 2, 3);
    progress.on_error(&repo, "fetch failed", 3, 3);
    progress.on_skip(&repo, "dirty tree", 3, 3);

    match expect_backend_event(rx.try_recv().expect("started event")) {
        BackendMessage::RepoStarted { repo_name } => assert_eq!(repo_name, repo.full_name()),
        _ => panic!("expected RepoStarted"),
    }

    match expect_backend_event(rx.try_recv().expect("fetch event")) {
        BackendMessage::RepoProgress {
            success,
            skipped,
            message,
            is_clone,
            had_updates,
            new_commits,
            ..
        } => {
            assert!(success);
            assert!(!skipped);
            assert_eq!(message, "updated");
            assert!(!is_clone);
            assert!(had_updates);
            assert_eq!(new_commits, Some(5));
        }
        _ => panic!("expected RepoProgress (fetch)"),
    }

    match expect_backend_event(rx.try_recv().expect("pull event")) {
        BackendMessage::RepoProgress {
            success,
            message,
            is_clone,
            had_updates,
            ..
        } => {
            assert!(success);
            assert_eq!(message, "fast-forward");
            assert!(!is_clone);
            assert!(had_updates);
        }
        _ => panic!("expected RepoProgress (pull)"),
    }

    match expect_backend_event(rx.try_recv().expect("error event")) {
        BackendMessage::RepoProgress {
            success,
            skipped,
            message,
            is_clone,
            ..
        } => {
            assert!(!success);
            assert!(!skipped);
            assert_eq!(message, "fetch failed");
            assert!(!is_clone);
        }
        _ => panic!("expected RepoProgress (error)"),
    }

    match expect_backend_event(rx.try_recv().expect("skip event")) {
        BackendMessage::RepoProgress {
            success,
            skipped,
            message,
            skip_reason,
            ..
        } => {
            assert!(success);
            assert!(skipped);
            assert_eq!(message, "skipped: dirty tree");
            assert_eq!(skip_reason.as_deref(), Some("dirty tree"));
        }
        _ => panic!("expected RepoProgress (skip)"),
    }
}

#[tokio::test]
async fn spawn_operation_sync_without_workspace_emits_operation_error() {
    let mut app = App::new(Config::default(), Vec::new());
    app.active_workspace = None;

    let (tx, mut rx) = unbounded_channel();
    spawn_operation(Operation::Sync, &app, tx);

    let event = timeout(Duration::from_secs(1), rx.recv())
        .await
        .expect("timed out waiting for backend message")
        .expect("channel closed unexpectedly");

    match expect_backend_event(event) {
        BackendMessage::OperationError(msg) => {
            assert!(msg.contains("No workspace selected"));
        }
        _ => panic!("expected OperationError"),
    }
}

#[tokio::test]
async fn spawn_operation_status_without_workspace_emits_operation_error() {
    let mut app = App::new(Config::default(), Vec::new());
    app.active_workspace = None;

    let (tx, mut rx) = unbounded_channel();
    spawn_operation(Operation::Status, &app, tx);

    let event = timeout(Duration::from_secs(1), rx.recv())
        .await
        .expect("timed out waiting for backend message")
        .expect("channel closed unexpectedly");

    match expect_backend_event(event) {
        BackendMessage::OperationError(msg) => {
            assert!(msg.contains("No workspace selected"));
        }
        _ => panic!("expected OperationError"),
    }
}
