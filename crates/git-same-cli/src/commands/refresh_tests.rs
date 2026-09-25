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

/// Binds the monitor socket for `ipc` and answers one request with `answer`,
/// or never accepts when `answer` is `None` (a monitor busy scanning).
#[cfg(unix)]
async fn fake_monitor(
    ipc: &git_same_core::ipc::IpcConfig,
    answer: Option<&'static str>,
) -> tokio::task::JoinHandle<()> {
    use git_same_core::ipc::unix_socket::write_response;
    use git_same_core::ipc::UnixSocketListener;
    use tokio::io::{AsyncBufReadExt, BufReader};

    std::fs::create_dir_all(&ipc.dir).unwrap();
    let listener = UnixSocketListener::new(ipc.socket_path())
        .bind()
        .await
        .unwrap();
    tokio::spawn(async move {
        let Some(answer) = answer else {
            std::future::pending::<()>().await;
            drop(listener);
            return;
        };
        let (stream, _) = listener.accept().await.unwrap();
        let mut reader = BufReader::new(stream);
        let mut line = String::new();
        reader.read_line(&mut line).await.unwrap();
        write_response(reader.get_mut(), answer).await.unwrap();
    })
}

#[cfg(unix)]
fn short_ipc() -> (tempfile::TempDir, git_same_core::ipc::IpcConfig) {
    // Short path: macOS caps socket paths at 104 bytes.
    let temp = tempfile::Builder::new()
        .prefix("gsr")
        .tempdir_in("/tmp")
        .unwrap();
    let ipc = git_same_core::ipc::IpcConfig {
        dir: temp.path().join("ipc"),
    };
    (temp, ipc)
}

#[cfg(unix)]
#[tokio::test]
async fn refresh_succeeds_when_the_monitor_queues_the_request() {
    let (_temp, ipc) = short_ipc();
    let server = fake_monitor(&ipc, Some("OK\n")).await;

    let args = RefreshArgs { path: None };
    assert!(run_with_ipc(&args, &Output::quiet(), &ipc).await.is_ok());
    server.await.unwrap();
}

#[cfg(unix)]
#[tokio::test]
async fn refresh_reports_a_busy_monitor_without_failing_or_hanging() {
    let (_temp, ipc) = short_ipc();
    let server = fake_monitor(&ipc, None).await;

    let args = RefreshArgs { path: None };
    let started = std::time::Instant::now();
    let res = run_with_timeout(
        &args,
        &Output::quiet(),
        &ipc,
        std::time::Duration::from_millis(100),
    )
    .await;
    assert!(res.is_ok(), "busy is not a failure: {res:?}");
    assert!(started.elapsed() < std::time::Duration::from_secs(5));
    server.abort();
}

#[cfg(unix)]
#[tokio::test]
async fn refresh_fails_when_the_monitor_answers_error() {
    let (_temp, ipc) = short_ipc();
    let server = fake_monitor(&ipc, Some("ERROR\n")).await;

    let args = RefreshArgs { path: None };
    assert!(run_with_ipc(&args, &Output::quiet(), &ipc).await.is_err());
    server.await.unwrap();
}
