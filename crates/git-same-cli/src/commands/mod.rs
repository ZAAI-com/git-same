//! Command handlers for the CLI subcommands.
//!
//! This module contains the runtime behavior for each subcommand,
//! separated from `main.rs` so the entrypoint stays focused on bootstrapping.

pub mod auto_monitor;
pub mod init;
pub mod monitor;
pub mod refresh;
pub mod reset;
pub mod scan;
#[cfg(feature = "tui")]
pub mod setup;
pub mod status;
pub mod support;
pub mod sync_cmd;
pub mod workspace;

pub use init::run as run_init;
pub use status::run as run_status;
pub use sync_cmd::run as run_sync_cmd;

use crate::cli::Command;
use auto_monitor::HookTiming;
use git_same_core::config::Config;
use git_same_core::errors::{AppError, Result};
use git_same_core::output::Output;
use std::path::Path;

pub(crate) use support::{ensure_base_path, warn_if_concurrency_capped};

/// Run the specified command.
pub async fn run_command(
    command: &Command,
    config_path: Option<&Path>,
    output: &Output,
    quiet: bool,
) -> Result<()> {
    // launchd and Homebrew invocations log to files: no banner for them.
    let private_monitor_mode = matches!(command, Command::Monitor(args) if args.is_private());
    if !output.is_json() && !quiet && !private_monitor_mode {
        crate::banner::print_banner();
    }

    // The monitor never needs a valid repository configuration: controls
    // work without one, and the loop falls back to defaults.
    if let Command::Monitor(args) = command {
        return if args.is_control() {
            monitor::run_control(args, config_path, output).await
        } else {
            monitor::run_foreground(args, config_path, output).await
        };
    }

    let timing = auto_monitor::hook_timing(command, config_path.is_some());
    let hook_output = auto_monitor::HookOutput {
        json: output.is_json(),
        quiet,
    };
    if timing == HookTiming::Before {
        auto_monitor::ensure(false, hook_output).await;
    }

    let registry_changed = dispatch(command, config_path, output).await?;

    if timing == HookTiming::AfterThenRefresh && registry_changed {
        auto_monitor::ensure(false, hook_output).await;
        auto_monitor::request_refresh().await;
    }
    Ok(())
}

/// Runs the handler. Returns whether the default configuration or workspace
/// registry may have changed, which is what the "after" hook acts on.
async fn dispatch(command: &Command, config_path: Option<&Path>, output: &Output) -> Result<bool> {
    // Commands that don't need config
    match command {
        Command::Init(args) => return run_init(args, output).await.map(|()| true),
        Command::Reset(args) => {
            return reset::run(args, output, &stop_monitor_before_reset)
                .await
                .map(|()| false)
        }
        Command::Scan(args) => return scan::run(args, config_path, output).map(|n| n > 0),
        #[cfg(feature = "tui")]
        Command::Setup(args) => return setup::run(args, output).await,
        _ => {}
    }

    // Load config for all other commands
    let config = load_config(config_path)?;

    match command {
        Command::Init(_) | Command::Reset(_) | Command::Scan(_) | Command::Monitor(_) => {
            unreachable!()
        }
        #[cfg(feature = "tui")]
        Command::Setup(_) => unreachable!(),
        Command::Sync(args) => run_sync_cmd(args, &config, output).await,
        Command::Status(args) => run_status(args, &config, output).await,
        Command::Workspace(args) => workspace::run(args, &config, output),
        Command::Refresh(args) => refresh::run(args, &config, output).await,
    }
    .map(|()| false)
}

/// Deleting the global configuration would also delete the stop preference,
/// so monitoring is stopped persistently (in launchd) first. Skipped without
/// complaint when this is not the real user's default environment.
fn stop_monitor_before_reset() {
    use git_same_core::macos::monitor_agent;
    if monitor_agent::autostart_suppressed() {
        return;
    }
    if let Ok(controller) = monitor_agent::controller_for_current_user(false) {
        if let Err(error) = controller.stop() {
            tracing::debug!(%error, "Could not stop the monitor before reset");
        }
    }
}

/// Load configuration from the given path or default location.
///
/// Returns an error suggesting `gisa init` when no config file exists
/// at the default location, rather than silently using defaults.
fn load_config(config_path: Option<&Path>) -> Result<Config> {
    let path = match config_path {
        Some(p) => p.to_path_buf(),
        None => Config::default_path()?,
    };
    if !path.exists() {
        return Err(AppError::config(
            "No configuration found. Run 'gisa init' to create one.",
        ));
    }
    Config::load_from(&path)
}
