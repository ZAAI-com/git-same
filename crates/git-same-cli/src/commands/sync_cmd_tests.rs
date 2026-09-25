use super::*;
use git_same_core::output::{Output, Verbosity};

fn default_args() -> SyncCmdArgs {
    SyncCmdArgs {
        workspace: None,
        pull: false,
        dry_run: false,
        concurrency: None,
        refresh: false,
        no_skip_uncommitted: false,
    }
}

#[tokio::test]
async fn run_returns_error_when_no_workspace_is_configured() {
    let _lock = crate::test_support::ENV_LOCK.lock().await;
    let original_home = std::env::var("HOME").ok();
    let temp = tempfile::tempdir().unwrap();
    std::env::set_var("HOME", temp.path());

    // resolve() walks up from cwd looking for any `.git-same/` directory, so we
    // must run from an isolated cwd to avoid matching a real workspace above the
    // repo on a developer machine.
    let original_cwd = std::env::current_dir().unwrap();
    std::env::set_current_dir(temp.path()).unwrap();
    struct CwdRestore(std::path::PathBuf);
    impl Drop for CwdRestore {
        fn drop(&mut self) {
            let _ = std::env::set_current_dir(&self.0);
        }
    }
    let _cwd_restore = CwdRestore(original_cwd);

    let args = default_args();
    let config = Config::default();
    let output = Output::new(Verbosity::Quiet, false);

    let result = run(&args, &config, &output).await;

    if let Some(home) = original_home {
        std::env::set_var("HOME", home);
    } else {
        std::env::remove_var("HOME");
    }

    let err = result.unwrap_err();
    assert!(err.to_string().contains("No workspaces configured"));
}

#[tokio::test]
async fn run_returns_error_for_unknown_workspace_name() {
    let _lock = crate::test_support::ENV_LOCK.lock().await;
    let original_home = std::env::var("HOME").ok();
    let temp = tempfile::tempdir().unwrap();
    std::env::set_var("HOME", temp.path());

    let mut args = default_args();
    args.workspace = Some("unknown-workspace".to_string());

    let config = Config::default();
    let output = Output::new(Verbosity::Quiet, false);

    let result = run(&args, &config, &output).await;

    if let Some(home) = original_home {
        std::env::set_var("HOME", home);
    } else {
        std::env::remove_var("HOME");
    }

    let err = result.unwrap_err();
    assert!(
        err.to_string().contains("No workspace configured")
            || err.to_string().contains("No workspace config found")
            || err.to_string().contains("Configuration error"),
        "unexpected error: {}",
        err
    );
}

#[test]
fn run_function_is_exposed() {
    let _fn_ptr = run;
}

mod summary {
    use super::super::*;
    use git_same_core::operations::clone::CloneResult;
    use git_same_core::operations::sync::SyncResult;
    use git_same_core::types::{OpResult, Repo};
    use std::path::PathBuf;

    fn repo(name: &str) -> OwnedRepo {
        OwnedRepo::new("org", Repo::test(name, "org"))
    }

    fn plain(lines: Vec<String>) -> Vec<String> {
        lines
            .iter()
            .map(|l| console::strip_ansi_codes(l).to_string())
            .collect()
    }

    fn sync_result(name: &str, result: OpResult, had_updates: bool) -> SyncResult {
        SyncResult {
            repo: repo(name),
            path: PathBuf::from("/tmp").join(name),
            result,
            had_updates,
            status: None,
            fetch_result: None,
            pull_result: None,
        }
    }

    fn outcome() -> SyncExecutionOutcome {
        let clone_results = vec![
            CloneResult {
                repo: repo("new"),
                path: PathBuf::from("/tmp/new"),
                result: OpResult::Success,
            },
            CloneResult {
                repo: repo("gone"),
                path: PathBuf::from("/tmp/gone"),
                result: OpResult::Failed("fatal: repository not found\nmore detail".into()),
            },
        ];
        let mut clone_summary = OpSummary::new();
        clone_results
            .iter()
            .for_each(|r| clone_summary.record(&r.result));

        let sync_results = vec![
            sync_result("a", OpResult::Success, true),
            sync_result("b", OpResult::Success, false),
            sync_result("c", OpResult::Failed("\n  error: timeout\n".into()), false),
        ];
        let mut sync_summary = OpSummary::new();
        sync_results
            .iter()
            .for_each(|r| sync_summary.record(&r.result));

        SyncExecutionOutcome::from_phases(
            Some((clone_summary, clone_results)),
            Some((sync_summary, sync_results)),
        )
    }

    #[test]
    fn failure_lines_list_each_failure_on_its_first_line_and_planning_skips() {
        let dirty = repo("dirty");
        let skipped = [(&dirty, "uncommitted changes")];
        assert_eq!(
            plain(failure_lines(&outcome(), &skipped)),
            vec![
                "  ✗ org/gone: fatal: repository not found",
                "  ✗ org/c: error: timeout",
                "  → org/dirty: uncommitted changes",
            ]
        );
    }

    #[test]
    fn failure_lines_are_empty_when_everything_succeeded() {
        let ok = SyncExecutionOutcome::from_phases(
            None,
            Some((
                OpSummary::new(),
                vec![sync_result("a", OpResult::Success, true)],
            )),
        );
        assert!(failure_lines(&ok, &[]).is_empty());
    }

    #[test]
    fn summary_lines_count_each_phase_and_planning_skips() {
        assert_eq!(
            plain(summary_lines(&outcome(), "Fetch", 5)),
            vec![
                "⚠ Cloned 1 of 2 new repositories, 1 failed",
                "⚠ Fetched 2 of 3 repositories (1 with updates), 1 failed",
                "→ Skipped 5 repositories",
            ]
        );
    }

    #[test]
    fn summary_lines_report_up_to_date_when_nothing_ran() {
        let idle = SyncExecutionOutcome::default();
        assert_eq!(
            plain(summary_lines(&idle, "Fetch", 0)),
            vec!["✓ All repositories are up to date"]
        );
    }
}
