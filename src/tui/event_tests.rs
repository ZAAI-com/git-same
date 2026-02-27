use super::*;
use crate::setup::state::OrgEntry;
use crate::tui::app::{CheckEntry, Operation, RepoEntry};
use crate::types::OpSummary;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::path::PathBuf;

fn sample_repo() -> OwnedRepo {
    OwnedRepo::new("acme", crate::types::Repo::test("rocket", "acme"))
}

fn sample_repo_entry() -> RepoEntry {
    RepoEntry {
        owner: "acme".to_string(),
        name: "rocket".to_string(),
        full_name: "acme/rocket".to_string(),
        path: PathBuf::from("/tmp/acme/rocket"),
        branch: Some("main".to_string()),
        is_uncommitted: false,
        ahead: 0,
        behind: 0,
        staged_count: 0,
        unstaged_count: 0,
        untracked_count: 0,
    }
}

#[test]
fn app_event_variants_construct() {
    let key = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);

    let terminal = AppEvent::Terminal(key);
    assert!(matches!(terminal, AppEvent::Terminal(_)));

    let resize = AppEvent::Resize(120, 40);
    assert!(matches!(resize, AppEvent::Resize(120, 40)));

    let backend = AppEvent::Backend(BackendMessage::OperationError("oops".to_string()));
    assert!(matches!(backend, AppEvent::Backend(_)));

    assert!(matches!(AppEvent::Tick, AppEvent::Tick));
}

#[test]
fn backend_message_variants_construct_and_clone() {
    let repo = sample_repo();
    let status_rows = vec![sample_repo_entry()];
    let checks = vec![CheckEntry {
        name: "git".to_string(),
        passed: true,
        message: "installed".to_string(),
        suggestion: None,
        critical: true,
    }];

    let msgs = vec![
        BackendMessage::OrgsDiscovered(1),
        BackendMessage::OrgStarted("acme".to_string()),
        BackendMessage::OrgComplete("acme".to_string(), 2),
        BackendMessage::DiscoveryComplete(vec![repo.clone()]),
        BackendMessage::DiscoveryError("err".to_string()),
        BackendMessage::SetupOrgsDiscovered(vec![OrgEntry {
            name: "acme".to_string(),
            repo_count: 2,
            selected: true,
        }]),
        BackendMessage::SetupOrgsError("err".to_string()),
        BackendMessage::OperationStarted {
            operation: Operation::Sync,
            total: 3,
            to_clone: 1,
            to_sync: 2,
        },
        BackendMessage::RepoStarted {
            repo_name: repo.full_name().to_string(),
        },
        BackendMessage::RepoProgress {
            repo_name: repo.full_name().to_string(),
            success: true,
            skipped: false,
            message: "ok".to_string(),
            had_updates: true,
            is_clone: false,
            new_commits: Some(3),
            skip_reason: None,
        },
        BackendMessage::RepoCommitLog {
            repo_name: repo.full_name().to_string(),
            commits: vec!["abc".to_string()],
        },
        BackendMessage::OperationComplete(OpSummary {
            success: 1,
            failed: 0,
            skipped: 0,
        }),
        BackendMessage::OperationError("err".to_string()),
        BackendMessage::StatusResults(status_rows),
        BackendMessage::SetupCheckResults(checks.clone()),
        BackendMessage::DefaultWorkspaceUpdated(Some("ws".to_string())),
        BackendMessage::DefaultWorkspaceError("bad".to_string()),
        BackendMessage::CheckResults(checks),
    ];

    for msg in msgs {
        let cloned = msg.clone();
        let dbg = format!("{:?}", cloned);
        assert!(!dbg.is_empty());
    }
}
