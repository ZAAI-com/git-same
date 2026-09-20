//! Service status model shared by the CLI and the desktop app.
//!
//! Process activity and badge-data freshness are separate concerns: a live
//! monitor whose `status.json` still comes from its predecessor is
//! `starting`, and an old scan timestamp never makes a running monitor
//! anything other than `running`.

use super::launchd::ServiceInfo;
use super::record::OwnerKind;
use crate::monitor::runtime_guard::{MonitorMode, RuntimeIdentity};
use serde::{Deserialize, Serialize};

/// Typed service state, exposed to TypeScript as a string union.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MonitorAgentState {
    /// Managed installation is absent.
    NotInstalled,
    /// Automatic monitoring was explicitly disabled.
    Disabled,
    /// Installation is prepared for the next GUI login.
    Deferred,
    /// Process exists, but current scan data is not ready.
    Starting,
    /// Active process has produced current status.
    Running,
    /// Enabled installation exists but no monitor is active.
    Stopped,
    /// A concrete installation or service error needs attention.
    Failed,
    /// Managed operation is unavailable on this platform.
    Unsupported,
}

/// Snapshot of the managed monitor service.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MonitorAgentStatus {
    pub label: String,
    pub plist_path: String,
    /// The managed helper executable, when installed.
    pub binary_path: Option<String>,
    pub installed: bool,
    pub loaded: bool,
    pub running: bool,
    pub state: MonitorAgentState,
    pub message: String,
    pub pid: Option<u32>,
    pub mode: Option<MonitorMode>,
    /// `monitor.autostart` from the default configuration.
    pub autostart: bool,
    /// launchd has the service explicitly disabled.
    pub launchd_disabled: bool,
    pub helper_version: Option<String>,
    /// App bundle or CLI the helper was installed from.
    pub source: Option<String>,
    pub owner_kind: Option<OwnerKind>,
    /// Timestamp of the last completed scan by the current process.
    pub last_scan: Option<String>,
    /// Error or required action, when there is one.
    pub detail: Option<String>,
}

/// Observations the state is derived from.
#[derive(Debug, Clone, Default)]
pub struct Facts {
    pub autostart: bool,
    pub launchd_disabled: bool,
    pub installed: bool,
    pub gui_session: bool,
    pub service: ServiceInfo,
    /// Verified holder of the runtime lock.
    pub active: Option<RuntimeIdentity>,
    /// The active process has written `status.json` itself.
    pub scan_complete: bool,
}

/// Pure state derivation.
pub fn derive_state(facts: &Facts) -> MonitorAgentState {
    if facts.active.is_some() {
        return if facts.scan_complete {
            MonitorAgentState::Running
        } else {
            MonitorAgentState::Starting
        };
    }
    if !facts.autostart || facts.launchd_disabled {
        return MonitorAgentState::Disabled;
    }
    if !facts.installed {
        return MonitorAgentState::NotInstalled;
    }
    if !facts.gui_session {
        return MonitorAgentState::Deferred;
    }
    // launchd has a process that has not taken the runtime lock yet.
    if facts.service.pid.is_some() {
        return MonitorAgentState::Starting;
    }
    MonitorAgentState::Stopped
}

/// Human-readable one-liner for a state.
pub fn state_message(state: MonitorAgentState, facts: &Facts) -> String {
    match state {
        MonitorAgentState::NotInstalled => "Background monitor is not installed".to_string(),
        MonitorAgentState::Disabled if !facts.autostart => {
            "Monitoring is stopped. Start it with 'gisa monitor --start'".to_string()
        }
        MonitorAgentState::Disabled => {
            "Monitoring is disabled in macOS (launchd). Start it with 'gisa monitor --start'"
                .to_string()
        }
        MonitorAgentState::Deferred => "Monitor will start when you log in".to_string(),
        MonitorAgentState::Starting => "Monitor is starting; initial scan in progress".to_string(),
        MonitorAgentState::Running => match facts.active.as_ref().map(|a| a.mode) {
            Some(MonitorMode::Foreground) => "Monitor is running in the foreground".to_string(),
            _ => "Monitor is running".to_string(),
        },
        MonitorAgentState::Stopped => "Monitor is installed but not running".to_string(),
        MonitorAgentState::Failed => "Monitor needs attention".to_string(),
        MonitorAgentState::Unsupported => {
            "The background monitor service is only available on macOS".to_string()
        }
    }
}

impl MonitorAgentStatus {
    /// Status for platforms without a managed service.
    pub fn unsupported() -> Self {
        let facts = Facts::default();
        Self {
            label: super::LABEL.to_string(),
            plist_path: String::new(),
            binary_path: None,
            installed: false,
            loaded: false,
            running: false,
            state: MonitorAgentState::Unsupported,
            message: state_message(MonitorAgentState::Unsupported, &facts),
            pid: None,
            mode: None,
            autostart: true,
            launchd_disabled: false,
            helper_version: None,
            source: None,
            owner_kind: None,
            last_scan: None,
            detail: None,
        }
    }

    /// Marks the status as failed with a concrete reason.
    pub fn failed(mut self, detail: impl Into<String>) -> Self {
        self.state = MonitorAgentState::Failed;
        self.message = "Monitor needs attention".to_string();
        self.detail = Some(detail.into());
        self
    }
}

#[cfg(test)]
#[path = "status_tests.rs"]
mod tests;
