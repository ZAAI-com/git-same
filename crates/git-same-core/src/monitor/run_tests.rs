use super::*;

fn ipc_at(dir: PathBuf) -> IpcConfig {
    IpcConfig { dir }
}

fn options(dir: PathBuf) -> Options {
    Options {
        interval: Duration::from_secs(3600),
        ipc_config: ipc_at(dir),
    }
}

fn context(mode: MonitorMode) -> RunContext {
    RunContext {
        mode,
        config_path: None,
        interval_explicit: true,
    }
}

/// launchd restarts the helper after every unsuccessful exit, so a startup
/// failure a restart cannot fix must exit 0. An IPC directory that cannot be
/// created is exactly that: respawning every ten seconds fixes nothing.
#[tokio::test]
async fn managed_startup_exits_successfully_when_the_ipc_directory_is_unusable() {
    let dir = tempfile::tempdir().unwrap();
    let blocker = dir.path().join("blocker");
    std::fs::write(&blocker, b"not a directory").unwrap();

    let result = run_with(
        &Config::default(),
        &Output::quiet(),
        options(blocker.join("group-container")),
        context(MonitorMode::Managed),
        std::future::pending::<()>(),
    )
    .await;

    assert!(
        result.is_ok(),
        "managed helper must not ask to be restarted"
    );
}

/// A foreground monitor was started by hand: the user must see the failure.
#[tokio::test]
async fn foreground_startup_reports_an_unusable_ipc_directory() {
    let dir = tempfile::tempdir().unwrap();
    let blocker = dir.path().join("blocker");
    std::fs::write(&blocker, b"not a directory").unwrap();

    let result = run_with(
        &Config::default(),
        &Output::quiet(),
        options(blocker.join("group-container")),
        context(MonitorMode::Foreground),
        std::future::pending::<()>(),
    )
    .await;

    assert!(result.is_err());
}

/// An unwritable status file (here: the path is occupied by a directory) is
/// likewise permanent. Same contract.
#[tokio::test]
async fn managed_startup_exits_successfully_when_the_status_file_cannot_be_written() {
    let dir = tempfile::tempdir().unwrap();
    let ipc_dir = dir.path().join("group-container");
    std::fs::create_dir_all(&ipc_dir).unwrap();
    std::fs::create_dir_all(ipc_at(ipc_dir.clone()).status_file_path()).unwrap();

    let result = run_with(
        &Config::default(),
        &Output::quiet(),
        options(ipc_dir),
        context(MonitorMode::Managed),
        std::future::pending::<()>(),
    )
    .await;

    assert!(
        result.is_ok(),
        "managed helper must not ask to be restarted"
    );
}
