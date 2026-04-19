use super::*;

#[test]
fn test_is_process_alive_self() {
    let pid = std::process::id();
    assert!(is_process_alive(pid));
}

#[test]
fn test_is_process_alive_nonexistent() {
    // PID 99999 is very unlikely to exist
    assert!(!is_process_alive(99999));
}
