use super::*;
use crate::git::{FetchResult, PullResult};
use crate::operations::clone::CloneProgress;
use crate::operations::sync::SyncProgress;
use crate::provider::DiscoveryProgress;
use crate::types::{OwnedRepo, Repo};
use std::sync::{Arc, Mutex};

fn sample_repo() -> OwnedRepo {
    OwnedRepo::new("acme", Repo::test("rocket", "acme"))
}

fn reporter_with_events() -> (ProgressReporter, Arc<Mutex<Vec<ProgressEvent>>>) {
    let events = Arc::new(Mutex::new(Vec::new()));
    let captured = events.clone();
    let reporter = ProgressReporter::new(move |event| {
        captured.lock().unwrap().push(event);
    });
    (reporter, events)
}

#[test]
fn discovery_progress_emits_serializable_events() {
    let (reporter, events) = reporter_with_events();

    DiscoveryProgress::on_orgs_discovered(&reporter, 2);
    DiscoveryProgress::on_org_started(&reporter, "acme");
    DiscoveryProgress::on_org_complete(&reporter, "acme", 3);
    DiscoveryProgress::on_personal_repos_started(&reporter);
    DiscoveryProgress::on_personal_repos_complete(&reporter, 1);
    DiscoveryProgress::on_error(&reporter, "rate limited");

    let events = events.lock().unwrap();
    assert_eq!(
        events.as_slice(),
        [
            ProgressEvent::DiscoveryOrgsDiscovered { count: 2 },
            ProgressEvent::DiscoveryOrgStarted {
                org_name: "acme".to_string(),
            },
            ProgressEvent::DiscoveryOrgComplete {
                org_name: "acme".to_string(),
                repo_count: 3,
            },
            ProgressEvent::DiscoveryPersonalReposStarted,
            ProgressEvent::DiscoveryPersonalReposComplete { count: 1 },
            ProgressEvent::DiscoveryError {
                message: "rate limited".to_string(),
            },
        ]
    );
    assert!(serde_json::to_string(&events[0])
        .unwrap()
        .contains("discovery_orgs_discovered"));
}

#[test]
fn clone_progress_emits_repo_events() {
    let repo = sample_repo();
    let (reporter, events) = reporter_with_events();

    CloneProgress::on_start(&reporter, &repo, 0, 4);
    CloneProgress::on_complete(&reporter, &repo, 1, 4);
    CloneProgress::on_error(&reporter, &repo, "failed", 2, 4);
    CloneProgress::on_skip(&reporter, &repo, "exists", 3, 4);

    let events = events.lock().unwrap();
    assert_eq!(
        events.as_slice(),
        [
            ProgressEvent::CloneStarted {
                repo_name: "acme/rocket".to_string(),
                index: 0,
                total: 4,
            },
            ProgressEvent::CloneCompleted {
                repo_name: "acme/rocket".to_string(),
                index: 1,
                total: 4,
            },
            ProgressEvent::CloneFailed {
                repo_name: "acme/rocket".to_string(),
                error: "failed".to_string(),
                index: 2,
                total: 4,
            },
            ProgressEvent::CloneSkipped {
                repo_name: "acme/rocket".to_string(),
                reason: "exists".to_string(),
                index: 3,
                total: 4,
            },
        ]
    );
}

#[test]
fn sync_progress_emits_fetch_and_pull_details() {
    let repo = sample_repo();
    let (reporter, events) = reporter_with_events();
    let path = std::path::Path::new("/tmp/acme/rocket");

    SyncProgress::on_start(&reporter, &repo, path, 0, 3);
    SyncProgress::on_fetch_complete(
        &reporter,
        &repo,
        &FetchResult {
            updated: true,
            new_commits: Some(2),
        },
        1,
        3,
    );
    SyncProgress::on_pull_complete(
        &reporter,
        &repo,
        &PullResult {
            success: true,
            updated: true,
            fast_forward: true,
            error: None,
        },
        2,
        3,
    );

    let events = events.lock().unwrap();
    assert_eq!(
        events.as_slice(),
        [
            ProgressEvent::SyncStarted {
                repo_name: "acme/rocket".to_string(),
                path: "/tmp/acme/rocket".to_string(),
                index: 0,
                total: 3,
            },
            ProgressEvent::SyncFetched {
                repo_name: "acme/rocket".to_string(),
                updated: true,
                new_commits: Some(2),
                index: 1,
                total: 3,
            },
            ProgressEvent::SyncPulled {
                repo_name: "acme/rocket".to_string(),
                success: true,
                updated: true,
                fast_forward: true,
                error: None,
                index: 2,
                total: 3,
            },
        ]
    );
}
