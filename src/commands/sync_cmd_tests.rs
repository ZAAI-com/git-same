use super::*;
use crate::output::{Output, Verbosity};
use std::sync::Mutex;

static HOME_LOCK: Mutex<()> = Mutex::new(());

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
    let _lock = HOME_LOCK.lock().expect("HOME lock poisoned");
    let original_home = std::env::var("HOME").ok();
    let temp = tempfile::tempdir().unwrap();
    std::env::set_var("HOME", temp.path());

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
    let _lock = HOME_LOCK.lock().expect("HOME lock poisoned");
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
    assert!(err.to_string().contains("No workspace configured"));
}

#[test]
fn run_function_is_exposed() {
    let _fn_ptr = run;
}
