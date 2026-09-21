use super::*;
use crate::macos::monitor_agent::system::RealSystem;

#[test]
fn automatic_callers_do_not_wait_behind_an_operation() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("monitor").join("control.lock");
    let _held = ControlLock::acquire(&path, Wait::No, &RealSystem).unwrap();

    let error = ControlLock::acquire(&path, Wait::No, &RealSystem).unwrap_err();
    assert!(matches!(error, MonitorAgentError::Busy));
}

#[test]
fn explicit_callers_wait_a_bounded_time_then_report_busy() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("control.lock");
    let _held = ControlLock::acquire(&path, Wait::No, &RealSystem).unwrap();

    let started = std::time::Instant::now();
    let error = ControlLock::acquire(&path, Wait::UpTo(Duration::from_millis(300)), &RealSystem)
        .unwrap_err();
    assert!(matches!(error, MonitorAgentError::Busy));
    assert!(started.elapsed() >= Duration::from_millis(250));
}

#[test]
fn waiting_caller_proceeds_once_the_holder_finishes() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("control.lock");
    let held = ControlLock::acquire(&path, Wait::No, &RealSystem).unwrap();

    let waiter_path = path.clone();
    let waiter = std::thread::spawn(move || {
        ControlLock::acquire(
            &waiter_path,
            Wait::UpTo(Duration::from_secs(5)),
            &RealSystem,
        )
        .is_ok()
    });
    std::thread::sleep(Duration::from_millis(200));
    drop(held);
    assert!(waiter.join().unwrap());
    assert!(path.exists(), "lock file must survive release");
}

#[cfg(unix)]
#[test]
fn managed_root_is_private() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("monitor");
    let _lock = ControlLock::acquire(&root.join("control.lock"), Wait::No, &RealSystem).unwrap();
    let mode = std::fs::metadata(&root).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o700);
}
