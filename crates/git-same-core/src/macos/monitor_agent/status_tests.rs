use super::*;
use std::path::PathBuf;

fn enabled() -> Facts {
    Facts {
        autostart: true,
        installed: true,
        gui_session: true,
        ..Facts::default()
    }
}

fn identity(mode: MonitorMode) -> RuntimeIdentity {
    RuntimeIdentity {
        pid: 42,
        start_identity: None,
        executable: PathBuf::new(),
        mode,
        started_at: String::new(),
    }
}

#[test]
fn live_process_without_its_own_status_is_starting_not_failed() {
    let facts = Facts {
        active: Some(identity(MonitorMode::Managed)),
        scan_complete: false,
        ..enabled()
    };
    assert_eq!(derive_state(&facts), MonitorAgentState::Starting);
}

#[test]
fn live_process_with_its_own_status_is_running() {
    let facts = Facts {
        active: Some(identity(MonitorMode::Managed)),
        scan_complete: true,
        ..enabled()
    };
    assert_eq!(derive_state(&facts), MonitorAgentState::Running);
}

#[test]
fn foreground_monitor_counts_as_running_even_when_autostart_is_off() {
    let facts = Facts {
        autostart: false,
        active: Some(identity(MonitorMode::Foreground)),
        scan_complete: true,
        ..enabled()
    };
    assert_eq!(derive_state(&facts), MonitorAgentState::Running);
    assert!(state_message(MonitorAgentState::Running, &facts).contains("foreground"));
}

#[test]
fn either_disable_mechanism_yields_disabled() {
    let by_preference = Facts {
        autostart: false,
        ..enabled()
    };
    let by_launchd = Facts {
        launchd_disabled: true,
        ..enabled()
    };
    assert_eq!(derive_state(&by_preference), MonitorAgentState::Disabled);
    assert_eq!(derive_state(&by_launchd), MonitorAgentState::Disabled);
    assert_ne!(
        state_message(MonitorAgentState::Disabled, &by_preference),
        state_message(MonitorAgentState::Disabled, &by_launchd)
    );
}

#[test]
fn remaining_states_follow_installation_and_session() {
    assert_eq!(
        derive_state(&Facts {
            installed: false,
            ..enabled()
        }),
        MonitorAgentState::NotInstalled
    );
    assert_eq!(
        derive_state(&Facts {
            gui_session: false,
            ..enabled()
        }),
        MonitorAgentState::Deferred
    );
    assert_eq!(derive_state(&enabled()), MonitorAgentState::Stopped);

    let launched = Facts {
        service: ServiceInfo {
            loaded: true,
            pid: Some(7),
            program: None,
        },
        ..enabled()
    };
    assert_eq!(derive_state(&launched), MonitorAgentState::Starting);
}

#[test]
fn state_serializes_as_snake_case_strings() {
    let json = serde_json::to_string(&MonitorAgentState::NotInstalled).unwrap();
    assert_eq!(json, "\"not_installed\"");
    let status = MonitorAgentStatus::unsupported();
    assert_eq!(status.state, MonitorAgentState::Unsupported);
    let failed = status.failed("boom");
    assert_eq!(failed.state, MonitorAgentState::Failed);
    assert_eq!(failed.detail.as_deref(), Some("boom"));
}
