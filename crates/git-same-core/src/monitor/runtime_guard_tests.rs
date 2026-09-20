use super::*;

fn ipc(dir: &tempfile::TempDir) -> IpcConfig {
    IpcConfig {
        dir: dir.path().join("ipc"),
    }
}

#[test]
fn acquire_writes_identity_and_drop_removes_it() {
    let dir = tempfile::tempdir().unwrap();
    let ipc = ipc(&dir);

    let guard = RuntimeGuard::acquire(&ipc, MonitorMode::Foreground).unwrap();
    assert_eq!(guard.identity().pid, std::process::id());
    assert_eq!(read_identity(&ipc).as_ref(), Some(guard.identity()));
    assert!(lock_is_held(&ipc));
    assert_eq!(active_monitor(&ipc).unwrap().mode, MonitorMode::Foreground);

    drop(guard);
    assert!(!lock_is_held(&ipc));
    assert!(read_identity(&ipc).is_none());
    assert!(active_monitor(&ipc).is_none());
    assert!(
        ipc.runtime_lock_path().exists(),
        "lock file inode must survive"
    );
}

#[test]
fn second_acquire_reports_the_holder() {
    let dir = tempfile::tempdir().unwrap();
    let ipc = ipc(&dir);
    let _first = RuntimeGuard::acquire(&ipc, MonitorMode::Managed).unwrap();

    match RuntimeGuard::acquire(&ipc, MonitorMode::Foreground) {
        Err(AcquireError::Held(Some(holder))) => {
            assert_eq!(holder.mode, MonitorMode::Managed);
            assert_eq!(holder.pid, std::process::id());
        }
        other => panic!("expected Held, got {other:?}"),
    }
    // The loser must not have clobbered the winner's record.
    assert_eq!(read_identity(&ipc).unwrap().mode, MonitorMode::Managed);
}

#[test]
fn lock_is_reusable_after_release() {
    let dir = tempfile::tempdir().unwrap();
    let ipc = ipc(&dir);
    drop(RuntimeGuard::acquire(&ipc, MonitorMode::Managed).unwrap());
    let again = RuntimeGuard::acquire(&ipc, MonitorMode::Foreground).unwrap();
    assert_eq!(again.identity().mode, MonitorMode::Foreground);
}

#[test]
fn stale_record_without_lock_is_not_an_active_monitor() {
    let dir = tempfile::tempdir().unwrap();
    let ipc = ipc(&dir);
    std::fs::create_dir_all(&ipc.dir).unwrap();
    let stale = RuntimeIdentity {
        pid: std::process::id(),
        start_identity: None,
        executable: PathBuf::from("/nonexistent/git-same"),
        mode: MonitorMode::Managed,
        started_at: "2026-01-01T00:00:00Z".to_string(),
    };
    std::fs::write(
        ipc.runtime_identity_path(),
        serde_json::to_vec(&stale).unwrap(),
    )
    .unwrap();

    assert!(active_monitor(&ipc).is_none());
}

#[test]
fn reused_pid_with_different_start_identity_is_rejected() {
    let identity = RuntimeIdentity {
        pid: std::process::id(),
        start_identity: Some("not-the-real-start".to_string()),
        executable: PathBuf::new(),
        mode: MonitorMode::Managed,
        started_at: String::new(),
    };
    let expected = process::start_identity(std::process::id()).is_none();
    assert_eq!(identity_matches_live_process(&identity), expected);
}

#[test]
fn dead_pid_is_rejected() {
    let identity = RuntimeIdentity {
        pid: u32::MAX,
        start_identity: None,
        executable: PathBuf::new(),
        mode: MonitorMode::Foreground,
        started_at: String::new(),
    };
    assert!(!identity_matches_live_process(&identity));
}
