//! Managed background monitor: installation, startup, and recovery.
//!
//! One controller serves the Homebrew cask installer, the CLI, and the
//! desktop app. The monitor runs as a separate helper executable in the
//! user's Library under a per-user LaunchAgent, independent of
//! `Git-Same.app`.
//!
//! The module compiles on every platform so its logic is tested everywhere,
//! but [`UserContext::resolve`] only succeeds on macOS: elsewhere explicit
//! operations report `Unsupported` and automatic callers do nothing.
//!
//! Independent of Tauri, Svelte, clap, and ratatui.

pub mod context;
pub mod control_lock;
pub mod controller;
pub mod install;
pub mod launchd;
pub mod plist;
pub mod record;
pub mod source;
pub mod status;
pub mod system;

#[cfg(test)]
pub(crate) mod fake;

pub use context::{autostart_suppressed, MonitorAgentPaths, UserContext, DISABLE_AUTOSTART_ENV};
pub use controller::Controller;
pub use record::{InstallRecord, OwnerKind};
pub use source::HelperSource;
pub use status::{MonitorAgentState, MonitorAgentStatus};
pub use system::{RealSystem, System};

use crate::errors::MonitorAgentError;

/// launchd label of the monitor agent.
pub const LABEL: &str = "com.zaai.git-same.monitor";
/// Label used before the daemon was renamed to monitor (3.0.x).
pub const LEGACY_LABEL: &str = "com.zaai.git-same.daemon";
/// FinderSync identifier from before the 3.1.0 rename.
pub const OBSOLETE_FINDER_EXTENSION_ID: &str = "com.zaai.git-same.GitSameBadge.FinderSync";
/// Version of the private packaging interface (`--install-agent` and
/// friends) that the cask template is rendered against.
pub const PACKAGING_PROTOCOL_VERSION: u32 = 1;

/// Automatic recovery for app and CLI startup paths.
///
/// Returns `None` without touching anything when automatic management does
/// not apply: unsupported platform, suppressed by the environment, or not
/// the real user's default environment. Never enables a disabled service.
pub fn auto_ensure(config_override: bool) -> Option<Result<MonitorAgentStatus, MonitorAgentError>> {
    if autostart_suppressed() {
        return None;
    }
    let user = UserContext::resolve(config_override).ok()?;
    let caller = source::invoking_source().ok();
    Some(Controller::new(std::sync::Arc::new(RealSystem), user, caller).ensure_running())
}

/// Controller for explicit commands by the real current user.
pub fn controller_for_current_user(config_override: bool) -> Result<Controller, MonitorAgentError> {
    let user = UserContext::resolve(config_override)?;
    let caller = source::invoking_source().ok();
    Ok(Controller::new(
        std::sync::Arc::new(RealSystem),
        user,
        caller,
    ))
}
