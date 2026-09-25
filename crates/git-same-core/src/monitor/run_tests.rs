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

    let result = tokio::time::timeout(
        std::time::Duration::from_secs(2),
        run_with(
            &Config::default(),
            &Output::quiet(),
            options(blocker.join("group-container")),
            context(MonitorMode::Managed),
            std::future::pending::<()>(),
        ),
    )
    .await
    .expect("managed startup must not enter the monitor loop");

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

    let result = tokio::time::timeout(
        std::time::Duration::from_secs(2),
        run_with(
            &Config::default(),
            &Output::quiet(),
            options(blocker.join("group-container")),
            context(MonitorMode::Foreground),
            std::future::pending::<()>(),
        ),
    )
    .await
    .expect("foreground startup must not enter the monitor loop");

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

    let result = tokio::time::timeout(
        std::time::Duration::from_secs(2),
        run_with(
            &Config::default(),
            &Output::quiet(),
            options(ipc_dir),
            context(MonitorMode::Managed),
            std::future::pending::<()>(),
        ),
    )
    .await
    .expect("managed startup must not enter the monitor loop");

    assert!(
        result.is_ok(),
        "managed helper must not ask to be restarted"
    );
}

#[tokio::test]
async fn managed_startup_exits_successfully_when_runtime_lock_is_unusable() {
    let dir = tempfile::tempdir().unwrap();
    let ipc_dir = dir.path().join("group-container");
    let ipc = ipc_at(ipc_dir.clone());
    std::fs::create_dir_all(ipc.runtime_lock_path()).unwrap();

    let result = tokio::time::timeout(
        std::time::Duration::from_secs(2),
        run_with(
            &Config::default(),
            &Output::quiet(),
            options(ipc_dir),
            context(MonitorMode::Managed),
            std::future::pending::<()>(),
        ),
    )
    .await
    .expect("managed startup must not enter the monitor loop");

    assert!(result.is_ok(), "managed helper must not restart-loop");
}

#[test]
fn from_config_uses_config_interval_when_no_override() {
    let mut config = Config::default();
    config.monitor.fullscan_interval_secs = 90;

    let opts = Options::from_config(&config, ipc_at(PathBuf::from("/tmp/from-config")), None);

    assert_eq!(opts.interval, Duration::from_secs(90));
    assert_eq!(opts.ipc_config.dir, PathBuf::from("/tmp/from-config"));
}

#[test]
fn from_config_lets_explicit_override_win() {
    let mut config = Config::default();
    config.monitor.fullscan_interval_secs = 30;

    let opts = Options::from_config(&config, ipc_at(PathBuf::from("/tmp/from-config")), Some(10));

    assert_eq!(opts.interval, Duration::from_secs(10));
}

#[test]
fn only_badge_relevant_paths_inside_git_pass_the_watcher() {
    let repo = Path::new("/Users/me/ws/org/repo");
    let dropped = [
        ".git",
        ".git/index.lock",
        ".git/worktrees/x/index.lock",
        ".git/worktrees/tirana/conductor-checkpoint-tmp",
        ".git/refs/heads/main.lock",
        ".git/objects/ab/cdef",
        ".git/FETCH_HEAD",
        ".git/ORIG_HEAD",
        ".git/logs/HEAD",
        ".git/config.lock",
    ];
    let kept = [
        ".git/refs/heads/main",
        ".git/refs/remotes/origin/main",
        ".git/HEAD",
        ".git/index",
        ".git/config",
        ".git/packed-refs",
        ".git/logs/refs/stash",
        ".git/MERGE_HEAD",
        ".git/rebase-merge/done",
        ".git/worktrees/x/HEAD",
        ".git/worktrees/x/index",
        "src/main.rs",
        "Cargo.lock",
        ".gitignore",
        ".gitmodules",
        ".github/workflows/ci.yml",
    ];
    for rel in dropped {
        assert!(!is_badge_relevant(&repo.join(rel)), "{rel} must be dropped");
    }
    for rel in kept {
        assert!(is_badge_relevant(&repo.join(rel)), "{rel} must pass");
    }
}

#[test]
fn full_scan_delay_stretches_after_a_slow_pass() {
    let secs = Duration::from_secs;
    assert_eq!(full_scan_delay(secs(30), secs(130)), secs(520));
    assert_eq!(full_scan_delay(secs(30), secs(1)), secs(30));
    assert_eq!(
        full_scan_delay(secs(0), Duration::ZERO),
        MIN_FULLSCAN_INTERVAL
    );
    assert_eq!(full_scan_delay(secs(30), Duration::MAX), Duration::MAX);
}

#[test]
fn deadline_after_saturates_instead_of_panicking() {
    let now = Instant::now();
    assert!(deadline_after(now, Duration::MAX) > now);
    assert_eq!(
        deadline_after(now, Duration::from_secs(5)),
        now + Duration::from_secs(5)
    );
}

#[test]
fn repeated_full_requests_collapse_into_one_due_scan() {
    let now = Instant::now();
    let mut next_full_scan = now + Duration::from_secs(600);
    let mut pending = HashSet::new();
    let mut explicit = HashSet::new();
    for _ in 0..3 {
        apply_scan_request(
            ScanRequest::Full,
            now,
            &mut next_full_scan,
            &mut pending,
            &mut explicit,
        );
    }
    assert_eq!(next_full_scan, now, "one scan, due now");

    // A later request never pushes an already due scan back.
    apply_scan_request(
        ScanRequest::Full,
        now + Duration::from_secs(1),
        &mut next_full_scan,
        &mut pending,
        &mut explicit,
    );
    assert_eq!(next_full_scan, now);
    assert!(pending.is_empty() && explicit.is_empty());
}

#[test]
fn repo_requests_join_the_flush_as_explicit() {
    let now = Instant::now();
    let later = now + Duration::from_secs(60);
    let mut next_full_scan = later;
    let mut pending = HashSet::new();
    let mut explicit = HashSet::new();
    let repo = PathBuf::from("/tmp/repo");
    apply_scan_request(
        ScanRequest::Repo(repo.clone()),
        now,
        &mut next_full_scan,
        &mut pending,
        &mut explicit,
    );
    assert!(pending.contains(&repo));
    assert!(explicit.contains(&repo));
    assert_eq!(
        next_full_scan, later,
        "a repo request never forces a full scan"
    );
}
