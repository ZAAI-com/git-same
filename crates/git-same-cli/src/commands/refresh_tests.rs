use super::*;
use crate::cli::RefreshArgs;
use git_same_core::output::{Output, Verbosity};

#[tokio::test]
async fn refresh_with_no_monitor_returns_error_on_unix() {
    // With no monitor listening on the socket, the command must surface an
    // error (unlike the post-sync/post-reset nudges, which stay silent).
    let args = RefreshArgs { path: None };
    let output = Output::new(Verbosity::Quiet, false);

    #[cfg(unix)]
    {
        let temp = tempfile::tempdir().expect("tempdir");
        let ipc = git_same_core::ipc::IpcConfig {
            dir: temp.path().join("ipc"),
        };
        let res = run_with_ipc(&args, &output, &ipc).await;
        assert!(res.is_err(), "expected error when monitor is not running");
    }
    #[cfg(not(unix))]
    {
        let cfg = Config::default();
        let res = run(&args, &cfg, &output).await;
        assert!(res.is_ok(), "non-unix fallback should succeed");
    }
}

/// A monitor that holds the runtime lock but has not bound its socket yet is
/// still scanning: that is not "unreachable" and must not fail the command.
#[cfg(unix)]
#[tokio::test]
async fn refresh_during_the_initial_scan_succeeds() {
    use git_same_core::monitor::runtime_guard::{MonitorMode, RuntimeGuard};

    let temp = tempfile::tempdir().expect("tempdir");
    let ipc = git_same_core::ipc::IpcConfig {
        dir: temp.path().join("ipc"),
    };
    let _starting = RuntimeGuard::acquire(&ipc, MonitorMode::Managed).expect("runtime lock");

    let args = RefreshArgs { path: None };
    let output = Output::new(Verbosity::Quiet, true);
    assert!(run_with_ipc(&args, &output, &ipc).await.is_ok());
}

#[cfg(unix)]
#[tokio::test]
async fn refresh_fails_when_a_scanned_monitor_has_no_socket() {
    use git_same_core::monitor::runtime_guard::{MonitorMode, RuntimeGuard};
    use git_same_core::types::FinderStatus;

    let temp = tempfile::tempdir().expect("tempdir");
    let ipc = git_same_core::ipc::IpcConfig {
        dir: temp.path().join("ipc"),
    };
    let running = RuntimeGuard::acquire(&ipc, MonitorMode::Managed).expect("runtime lock");
    git_same_core::ipc::StatusFileWriter::new(ipc.status_file_path())
        .write(&FinderStatus::new(
            running.identity().pid,
            chrono::Utc::now().to_rfc3339(),
        ))
        .unwrap();

    let args = RefreshArgs { path: None };
    assert!(run_with_ipc(&args, &Output::quiet(), &ipc).await.is_err());
}
