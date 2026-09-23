use super::*;
use git_same_core::types::FinderStatus;

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

fn stopped_status() -> MonitorAgentStatus {
    MonitorAgentStatus {
        installed: true,
        state: MonitorAgentState::Stopped,
        message: "Monitor is installed but not running".to_string(),
        ..MonitorAgentStatus::unsupported()
    }
}

#[test]
fn cask_install_before_the_app_is_placed_says_when_the_monitor_starts() {
    let dir = tempfile::tempdir().unwrap();
    let app = dir.path().join("Git-Same.app");

    let report = install_agent_report(stopped_status(), &app);

    assert_eq!(
        report.headline,
        "Monitor installed; it starts when you open Git-Same or at next login"
    );
}

#[test]
fn cask_install_with_the_app_in_place_keeps_the_status_message() {
    let dir = tempfile::tempdir().unwrap();
    let app = dir.path().join("Git-Same.app");
    let executable = monitor_agent::source::app_main_executable(&app);
    std::fs::create_dir_all(executable.parent().unwrap()).unwrap();
    std::fs::write(&executable, b"app").unwrap();

    let report = install_agent_report(stopped_status(), &app);

    assert_eq!(report.headline, "Monitor is installed but not running");
}

#[test]
fn cask_install_with_monitoring_stopped_keeps_the_status_message() {
    let dir = tempfile::tempdir().unwrap();
    let status = MonitorAgentStatus {
        state: MonitorAgentState::Disabled,
        message: "Monitoring is stopped".to_string(),
        ..stopped_status()
    };

    let report = install_agent_report(status, &dir.path().join("Git-Same.app"));

    assert_eq!(report.headline, "Monitoring is stopped");
}
