//! Automatic monitor recovery for eligible CLI invocations.
//!
//! Hooks run only from the dispatcher (`run_command`) and the TUI entry in
//! `main.rs`, never from inside a command handler: handler unit tests call
//! handlers in-process and must not be able to reach launchd. The decision
//! is a pure function so the whole table is testable.

use crate::cli::Command;
use git_same_core::macos::monitor_agent::{self, MonitorAgentState};

/// When to recover the monitor relative to the command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookTiming {
    /// No automatic service changes.
    None,
    /// Ensure before the command runs.
    Before,
    /// Ensure after the command succeeded and changed the registry, then ask
    /// the monitor to reload: a healthy monitor is never restarted, so the
    /// refresh is what carries the change.
    AfterThenRefresh,
}

/// The automatic-hook table. `config_override` is `--config`.
pub fn hook_timing(command: &Command, config_override: bool) -> HookTiming {
    if config_override {
        return HookTiming::None;
    }
    match command {
        Command::Sync(args) if !args.dry_run => HookTiming::Before,
        Command::Refresh(_) => HookTiming::Before,
        // `init --path` writes somewhere other than the default configuration.
        Command::Init(args) if args.path.is_none() => HookTiming::AfterThenRefresh,
        Command::Scan(args) if args.register => HookTiming::AfterThenRefresh,
        #[cfg(feature = "tui")]
        Command::Setup(_) => HookTiming::AfterThenRefresh,
        // Diagnostics, dry runs, reset, and every monitor invocation.
        _ => HookTiming::None,
    }
}

/// How hook output is reported.
#[derive(Debug, Clone, Copy)]
pub struct HookOutput {
    pub json: bool,
    pub quiet: bool,
}

impl HookOutput {
    /// Recovery problems are warnings on stderr. The CLI's default verbosity
    /// already hides `Output::warn`, so this gate is what "quiet" means here.
    fn warn(&self, message: &str) {
        if !self.json && !self.quiet {
            eprintln!("warning: {message}");
        }
    }
}

/// Recovers enabled monitoring. Never fails the command it accompanies.
pub async fn ensure(config_override: bool, output: HookOutput) {
    let result =
        tokio::task::spawn_blocking(move || monitor_agent::auto_ensure(config_override)).await;
    match result {
        Ok(Some(Ok(status))) if status.state == MonitorAgentState::Failed => {
            output.warn(&format!(
                "background monitor: {}",
                status.detail.unwrap_or(status.message)
            ));
        }
        Ok(Some(Err(error))) => output.warn(&format!("background monitor: {error}")),
        _ => {}
    }
}

/// Asks a running monitor to reload the configuration and rescan.
#[cfg(unix)]
pub async fn request_refresh() {
    use git_same_core::ipc::{IpcConfig, UnixSocketClient};
    let Ok(ipc) = IpcConfig::default_path() else {
        return;
    };
    if let Err(e) = UnixSocketClient::new(ipc.socket_path()).refresh_all().await {
        tracing::debug!(error = %e, "Monitor refresh nudge skipped");
    }
}

#[cfg(not(unix))]
pub async fn request_refresh() {}

#[cfg(test)]
#[path = "auto_monitor_tests.rs"]
mod tests;
