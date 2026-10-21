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

/// Serializes the tests that install the global `BEFORE_JOB` hook.
static JOB_HOOK_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// How long a held job waits for release at most. Far longer than any
/// deadline the tests assert, so "returned in time" proves "did not wait".
const JOB_HOLD_LIMIT: std::time::Duration = std::time::Duration::from_secs(10);

/// Holds every scan job until the test releases it, and counts jobs.
struct HeldJobs {
    started: Arc<std::sync::atomic::AtomicUsize>,
    release: std::sync::mpsc::Sender<()>,
}

impl HeldJobs {
    fn install() -> Self {
        let started = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let (release, released) = std::sync::mpsc::channel::<()>();
        let released = std::sync::Mutex::new(released);
        let counter = started.clone();
        *BEFORE_JOB.lock().unwrap() = Some(Arc::new(move || {
            counter.fetch_add(1, Ordering::SeqCst);
            let _ = released.lock().unwrap().recv_timeout(JOB_HOLD_LIMIT);
        }));
        Self { started, release }
    }

    fn started(&self) -> usize {
        self.started.load(Ordering::SeqCst)
    }

    async fn wait_for(&self, jobs: usize) {
        for _ in 0..200 {
            if self.started() >= jobs {
                return;
            }
            tokio::time::sleep(std::time::Duration::from_millis(25)).await;
        }
        panic!("expected {jobs} scan jobs, saw {}", self.started());
    }
}

impl Drop for HeldJobs {
    fn drop(&mut self) {
        *BEFORE_JOB.lock().unwrap() = None;
    }
}

/// Starts a foreground monitor with no workspaces in a short temp dir and
/// returns its socket client, its stop switch, and its task.
async fn start_monitor() -> (
    tempfile::TempDir,
    crate::ipc::UnixSocketClient,
    tokio::sync::oneshot::Sender<()>,
    tokio::task::JoinHandle<Result<()>>,
) {
    let dir = tempfile::Builder::new()
        .prefix("gsm")
        .tempdir_in("/tmp")
        .unwrap();
    let ipc_dir = dir.path().join("ipc");
    let socket = ipc_at(ipc_dir.clone()).socket_path();
    let (stop, stopped) = tokio::sync::oneshot::channel::<()>();
    let task = tokio::spawn(async move {
        let mut config = Config::default();
        config.finder.show_ambient = false;
        run_with(
            &config,
            &Output::quiet(),
            options(ipc_dir),
            context(MonitorMode::Foreground),
            async {
                let _ = stopped.await;
            },
        )
        .await
    });
    for _ in 0..200 {
        if socket.exists() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
    }
    assert!(socket.exists(), "the monitor never bound its socket");
    (dir, crate::ipc::UnixSocketClient::new(socket), stop, task)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_loop_answers_clients_while_a_scan_runs_and_reruns_once() {
    use crate::ipc::Reply;
    let _serial = JOB_HOOK_LOCK.lock().await;
    let jobs = HeldJobs::install();
    let (_dir, client, stop, task) = start_monitor().await;
    let quick = std::time::Duration::from_millis(500);

    // Starts a full scan, which the hook holds open.
    assert_eq!(
        client.request("REFRESH_ALL", quick).await.unwrap(),
        Reply::Answered("OK\n".into())
    );
    jobs.wait_for(1).await;

    // The loop still answers while that scan runs.
    assert_eq!(
        client.request("PING", quick).await.unwrap(),
        Reply::Answered("PONG\n".into())
    );
    // Two more full requests during the scan collapse into one rerun.
    for _ in 0..2 {
        assert_eq!(
            client.request("REFRESH_ALL", quick).await.unwrap(),
            Reply::Answered("OK\n".into())
        );
    }

    jobs.release.send(()).unwrap();
    jobs.wait_for(2).await;
    jobs.release.send(()).unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    assert_eq!(jobs.started(), 2, "exactly one follow-up scan");

    stop.send(()).unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(2), task)
        .await
        .expect("monitor must stop promptly")
        .unwrap()
        .unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn shutdown_does_not_wait_for_a_running_scan() {
    let _serial = JOB_HOOK_LOCK.lock().await;
    let jobs = HeldJobs::install();
    let (_dir, client, stop, task) = start_monitor().await;

    client
        .notify("REFRESH_ALL", std::time::Duration::from_millis(500))
        .await
        .unwrap();
    jobs.wait_for(1).await;

    // The scan is still held (up to JOB_HOLD_LIMIT); the loop must return
    // well before that.
    stop.send(()).unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(2), task)
        .await
        .expect("shutdown must not wait for the scan")
        .unwrap()
        .unwrap();
    jobs.release.send(()).unwrap();
}

#[test]
fn rescan_pending_records_changed_and_explicit_repos_in_the_ambient_cache() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(tmp.path().join("repo/.git")).unwrap();
    let repo = std::fs::canonicalize(tmp.path().join("repo")).unwrap();

    let config = Config::default();
    let git = crate::git::MockGit::new();
    let cache = AmbientUpgradeCache::new();
    let service = RepoScanService::new(&git, &config);
    let mut status = FinderStatus::new(1, "2026-01-01T00:00:00Z".into());

    // First sight of the repo: it changes the status and is cached.
    let none = HashSet::new();
    assert!(rescan_pending(
        &service,
        &mut status,
        &cache,
        HashSet::from([repo.clone()]),
        &none
    ));
    assert!(cache.get(&repo).is_some());

    // Unchanged and not requested: nothing changes, and the cache is untouched.
    cache.remove(&repo);
    assert!(!rescan_pending(
        &service,
        &mut status,
        &cache,
        HashSet::from([repo.clone()]),
        &none
    ));
    assert!(cache.get(&repo).is_none());

    // Unchanged but named in `REFRESH <path>`: recorded anyway.
    let explicit = HashSet::from([repo.clone()]);
    assert!(!rescan_pending(
        &service,
        &mut status,
        &cache,
        HashSet::from([repo.clone()]),
        &explicit
    ));
    assert!(cache.get(&repo).is_some());

    // A cancelled service stops before the first repo.
    cache.remove(&repo);
    let cancelled =
        RepoScanService::new(&git, &config).with_cancel(Arc::new(AtomicBool::new(true)));
    assert!(!rescan_pending(
        &cancelled,
        &mut status,
        &cache,
        HashSet::from([repo.clone()]),
        &explicit
    ));
    assert!(cache.get(&repo).is_none());
}

#[test]
fn only_workspace_and_requested_ambient_repos_are_rescanned_on_file_changes() {
    use crate::types::finder_status::FinderWorkspaceInfo;

    let tmp = tempfile::tempdir().unwrap();
    let root = std::fs::canonicalize(tmp.path()).unwrap();
    let ws = root.join("ws");
    let workspace_repo = ws.join("org/a");
    let new_workspace_clone = ws.join("org/new");
    let ambient = root.join("amb/x");
    let unknown_ambient = root.join("amb/y");
    for dir in [
        &workspace_repo,
        &new_workspace_clone,
        &ambient,
        &unknown_ambient,
    ] {
        std::fs::create_dir_all(dir).unwrap();
    }

    let config = Config::default();
    let git = crate::git::MockGit::new();
    let service = RepoScanService::new(&git, &config);
    let mut status = FinderStatus::new(1, "2026-01-01T00:00:00Z".into());
    status.workspaces.push(FinderWorkspaceInfo {
        name: "ws".into(),
        root: ws.clone(),
        orgs: Vec::new(),
    });
    let mut workspace_entry = service.scan_ambient_repo(&workspace_repo);
    workspace_entry.workspace = Some("ws".into());
    status.repos.push(workspace_entry);
    status.repos.push(service.scan_ambient_repo(&ambient));

    let upgrades = AmbientUpgradeCache::new();
    let all = || {
        HashSet::from([
            workspace_repo.clone(),
            new_workspace_clone.clone(),
            ambient.clone(),
            unknown_ambient.clone(),
        ])
    };

    // Gray ambient repos, known or not, ignore file changes.
    let wanted = wanted_rescans(&status, &upgrades, all(), &HashSet::new());
    assert_eq!(
        wanted,
        HashSet::from([workspace_repo.clone(), new_workspace_clone.clone()])
    );

    // After a right-click upgraded it, an ambient repo follows file changes.
    upgrades.set(ambient.clone(), service.scan_ambient_repo(&ambient));
    let wanted = wanted_rescans(&status, &upgrades, all(), &HashSet::new());
    assert!(wanted.contains(&ambient));
    assert!(!wanted.contains(&unknown_ambient));

    // A `REFRESH <path>` for an unknown ambient repo is always honored.
    let explicit = HashSet::from([unknown_ambient.clone()]);
    let wanted = wanted_rescans(&status, &upgrades, all(), &explicit);
    assert!(wanted.contains(&unknown_ambient));
}
