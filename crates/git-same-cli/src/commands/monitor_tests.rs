use super::*;
use git_same_core::types::FinderStatus;

#[test]
fn cli_flag_overrides_config_interval() {
    assert_eq!(resolve_interval_secs(Some(10), 30), 10);
}

#[test]
fn config_interval_used_when_flag_absent() {
    assert_eq!(resolve_interval_secs(None, 90), 90);
}

#[test]
fn state_names_match_the_typescript_union() {
    assert_eq!(state_name(MonitorAgentState::NotInstalled), "not_installed");
    assert_eq!(state_name(MonitorAgentState::Starting), "starting");
}

#[test]
fn unmanaged_status_without_a_monitor_is_informative() {
    let dir = tempfile::tempdir().unwrap();
    let ipc = IpcConfig {
        dir: dir.path().join("ipc"),
    };
    let status = unmanaged_status(&ipc);
    assert!(!status.running);
    assert_eq!(status.message, "Monitor is not running");
    assert!(read_data_summary(&ipc).is_none());
}

#[test]
fn stale_status_file_pid_is_not_reported_as_a_running_monitor() {
    let dir = tempfile::tempdir().unwrap();
    let ipc = IpcConfig {
        dir: dir.path().join("ipc"),
    };
    StatusFileWriter::new(ipc.status_file_path())
        .write(&FinderStatus::new(std::process::id(), "then".to_string()))
        .unwrap();

    let status = unmanaged_status(&ipc);

    assert!(!status.running, "a status-file PID alone proves nothing");
    assert_eq!(read_data_summary(&ipc).unwrap().last_written, "then");
}

#[test]
fn lock_holder_is_starting_until_it_writes_its_own_status() {
    let dir = tempfile::tempdir().unwrap();
    let ipc = IpcConfig {
        dir: dir.path().join("ipc"),
    };
    let _guard = runtime_guard::RuntimeGuard::acquire(&ipc, MonitorMode::Foreground).unwrap();

    let starting = unmanaged_status(&ipc);
    assert!(starting.running);
    assert!(starting.message.contains("initial scan in progress"));

    StatusFileWriter::new(ipc.status_file_path())
        .write(&FinderStatus::new(std::process::id(), "now".to_string()))
        .unwrap();
    assert!(unmanaged_status(&ipc)
        .message
        .starts_with("Monitor is running"));
}
