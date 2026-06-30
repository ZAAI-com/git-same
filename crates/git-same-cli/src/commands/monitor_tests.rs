use super::*;

#[test]
fn test_is_process_alive_self() {
    let pid = std::process::id();
    assert!(is_process_alive(pid));
}

// Probing a specific not-running PID only works where we can actually signal
// processes; the non-Unix fallback optimistically assumes in-range PIDs are
// alive, so this assertion is Unix-only.
#[cfg(unix)]
#[test]
fn test_is_process_alive_nonexistent() {
    // PID 99999 is very unlikely to exist
    assert!(!is_process_alive(99999));
}

#[test]
fn test_is_process_alive_rejects_out_of_range_pid() {
    // u32::MAX is the staleness sentinel; as a signed pid_t it is -1, which must
    // never be reported alive (it would broadcast to a process group on Linux).
    assert!(!is_process_alive(u32::MAX));
    assert!(!is_process_alive(0));
}

#[test]
fn cli_flag_overrides_config_interval() {
    assert_eq!(resolve_interval_secs(Some(10), 30), 10);
}

#[test]
fn config_interval_used_when_flag_absent() {
    assert_eq!(resolve_interval_secs(None, 90), 90);
}
