//! Errors from managing the background monitor service.

use std::path::PathBuf;
use std::time::Duration;
use thiserror::Error;

/// Typed failures of the managed monitor lifecycle.
///
/// Adapted into CLI exit codes through [`super::AppError`] and into strings at
/// the Tauri boundary. UI state never depends on parsing these messages: the
/// status DTO carries the state.
#[derive(Error, Debug)]
pub enum MonitorAgentError {
    /// Configuration could not be read or persisted.
    #[error("Monitor configuration error: {0}")]
    Configuration(String),

    /// The caller is not operating on the real user's default environment.
    #[error("Refusing to manage the monitor service: {0}")]
    Override(String),

    /// No executable is available to install as the helper.
    #[error("No monitor helper source is available: {0}")]
    MissingSource(String),

    /// The source or installed helper failed validation.
    #[error("Invalid monitor executable '{}': {reason}", path.display())]
    InvalidExecutable { path: PathBuf, reason: String },

    /// Another lifecycle operation holds the control lock.
    #[error("Another monitor operation is in progress")]
    Busy,

    /// An external command did not finish in time.
    #[error("'{command}' timed out after {}s", timeout.as_secs())]
    CommandTimeout { command: String, timeout: Duration },

    /// launchd rejected an operation.
    #[error("launchctl {operation} failed (exit {code:?}): {detail}")]
    Launchd {
        operation: String,
        code: Option<i32>,
        detail: String,
    },

    /// Managed services are not available on this platform.
    #[error("The managed monitor service is only available on macOS")]
    Unsupported,

    /// The installation belongs to someone else, or ownership is unprovable.
    #[error("Monitor installation is not owned by the caller: {0}")]
    OwnershipMismatch(String),

    /// The runtime lock is held: a monitor is already running.
    #[error("A monitor is already running{}", pid.map(|p| format!(" (PID {p})")).unwrap_or_default())]
    AlreadyRunning { pid: Option<u32> },

    /// A foreground monitor is running and must be stopped first.
    #[error(
        "A foreground monitor (PID {pid}) is running; stop it before using the background service"
    )]
    ForegroundActive { pid: u32 },

    /// Helper replacement failed. `rollback` is set when restoring also failed.
    #[error("Monitor installation failed: {original}{}", rollback.as_ref().map(|r| format!("; rollback also failed: {r}")).unwrap_or_default())]
    Transaction {
        original: String,
        rollback: Option<String>,
    },

    /// Filesystem failure with context.
    #[error("{context}: {source}")]
    Io {
        context: String,
        #[source]
        source: std::io::Error,
    },
}

impl MonitorAgentError {
    /// Wraps an I/O error with a description of what was being attempted.
    pub fn io(context: impl Into<String>, source: std::io::Error) -> Self {
        MonitorAgentError::Io {
            context: context.into(),
            source,
        }
    }

    /// Returns `true` when retrying shortly may succeed.
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            MonitorAgentError::Busy | MonitorAgentError::CommandTimeout { .. }
        )
    }

    /// Process exit code for the CLI.
    pub fn exit_code(&self) -> i32 {
        match self {
            MonitorAgentError::Busy => 9,
            MonitorAgentError::Unsupported => 10,
            _ => 8,
        }
    }

    /// Returns a suggested action to resolve this error.
    pub fn suggested_action(&self) -> &'static str {
        match self {
            MonitorAgentError::Configuration(_) => {
                "Fix the syntax error in config.toml, then run 'gisa monitor --status'"
            }
            MonitorAgentError::Override(_) => {
                "Run the command as your own user without --config or GIT_SAME_CONFIG_DIR"
            }
            MonitorAgentError::MissingSource(_) | MonitorAgentError::InvalidExecutable { .. } => {
                "Reinstall Git-Same, then run 'gisa monitor --start'"
            }
            MonitorAgentError::Busy | MonitorAgentError::CommandTimeout { .. } => {
                "Wait a moment and try again"
            }
            MonitorAgentError::Launchd { .. } => {
                "Run 'gisa monitor --status' and check ~/Library/Logs/git-same"
            }
            MonitorAgentError::Unsupported => "Run 'gisa monitor' in the foreground instead",
            MonitorAgentError::OwnershipMismatch(_) => {
                "Run 'gisa monitor --uninstall' to remove the existing installation"
            }
            MonitorAgentError::AlreadyRunning { .. } => {
                "Run 'gisa monitor --status' to inspect the running monitor"
            }
            MonitorAgentError::ForegroundActive { .. } => {
                "Run 'gisa monitor --stop', then 'gisa monitor --start'"
            }
            MonitorAgentError::Transaction { .. } | MonitorAgentError::Io { .. } => {
                "Run 'gisa monitor --status', then 'gisa monitor --start' to repair"
            }
        }
    }
}

#[cfg(test)]
#[path = "monitor_agent_tests.rs"]
mod tests;
