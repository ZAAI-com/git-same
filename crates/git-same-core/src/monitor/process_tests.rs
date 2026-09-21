use super::*;

#[test]
fn invalid_pids_are_never_alive() {
    assert!(!is_valid_pid(0));
    assert!(!is_valid_pid(u32::MAX));
    assert!(!is_alive(0));
    assert!(!is_alive(u32::MAX));
}

#[test]
fn current_process_is_alive() {
    assert!(is_alive(std::process::id()));
}

#[test]
fn terminate_rejects_invalid_pids() {
    assert!(terminate(0).is_err());
    assert!(terminate(u32::MAX).is_err());
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
#[test]
fn start_identity_is_stable_for_the_current_process() {
    let first = start_identity(std::process::id()).expect("identity");
    let second = start_identity(std::process::id()).expect("identity");
    assert_eq!(first, second);
    assert!(start_identity(u32::MAX).is_none());
}

#[cfg(unix)]
#[test]
fn exited_child_is_not_alive() {
    let mut child = std::process::Command::new("true").spawn().unwrap();
    let pid = child.id();
    child.wait().unwrap();
    assert!(!is_alive(pid));
}
