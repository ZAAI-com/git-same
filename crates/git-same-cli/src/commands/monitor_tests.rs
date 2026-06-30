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

#[test]
fn cli_flag_overrides_config_interval() {
    assert_eq!(resolve_interval_secs(Some(10), 30), 10);
}

#[test]
fn config_interval_used_when_flag_absent() {
    assert_eq!(resolve_interval_secs(None, 90), 90);
}
