//! Headless monitor mode for the app binary.
//!
//! An app-owned LaunchAgent runs `Git-Same.app/Contents/MacOS/git-same-app
//! monitor --foreground --managed` instead of a copy of the CLI helper.
//! macOS TCC attributes a launchd-spawned process to its bundle only when the
//! executable is the bundle's `CFBundleExecutable`, so running the loop here
//! is what lets one Full Disk Access grant for "Git-Same" cover the monitor
//! too. A copied helper is a separate, path-based TCC identity that the grant
//! never reaches.
//!
//! Nothing Tauri or AppKit is touched on this path: no window, no Dock icon.
//! The loop and the managed-startup rules both live in
//! `git_same_core::monitor`; this file is the same thin shim the CLI
//! `monitor` subcommand is.

use git_same_core::config::Config;
use git_same_core::errors::{AppError, Result};
use git_same_core::ipc::IpcConfig;
use git_same_core::monitor;
use git_same_core::output::{Output, Verbosity};
use std::ffi::OsStr;

/// Whether argv selects monitor mode: `git-same-app monitor [flags]`.
/// Only the first argument is inspected.
pub(crate) fn is_monitor_invocation<I, S>(args: I) -> bool
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    args.into_iter()
        .nth(1)
        .is_some_and(|arg| arg.as_ref() == "monitor")
}

/// Whether argv carries `--managed`, which launchd's plist always passes.
/// Without it the process is a monitor a user started by hand.
fn is_managed<I, S>(args: I) -> bool
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    args.into_iter().any(|arg| arg.as_ref() == "--managed")
}

/// Run the monitor loop until SIGTERM or SIGINT. Returns the process exit
/// code: launchd's `KeepAlive` restarts the agent on a non-zero exit.
pub(crate) fn run() -> i32 {
    init_logging();
    match run_inner(is_managed(std::env::args_os())) {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("git-same-app monitor: {error}");
            1
        }
    }
}

fn run_inner(managed: bool) -> Result<()> {
    let output = Output::new(Verbosity::Quiet, false);
    let runtime = tokio::runtime::Runtime::new()
        .map_err(|error| AppError::config(format!("tokio runtime init failed: {error}")))?;
    if managed {
        // Identical startup rules to `gisa monitor --foreground --managed`:
        // respect the stored preference, never rewrite a broken config, and
        // exit successfully on anything a restart cannot fix.
        return runtime.block_on(monitor::run_managed(&output));
    }
    let config = Config::load()?;
    let ipc_config = IpcConfig::default_path()?;
    ipc_config.ensure_dir()?;
    let opts = monitor::Options::from_config(&config, ipc_config, None);
    runtime.block_on(monitor::run(
        &config,
        &output,
        opts,
        monitor::default_shutdown_signal(),
    ))
}

/// Same `GISA_LOG` contract as the CLI (`crates/git-same-cli/src/main.rs`):
/// the env filter selects the level, default `warn`, written to stderr so
/// launchd's `StandardErrorPath` captures it.
fn init_logging() {
    use tracing_subscriber::{fmt, prelude::*, EnvFilter};

    let filter = EnvFilter::try_from_env("GISA_LOG").unwrap_or_else(|_| EnvFilter::new("warn"));
    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().with_writer(std::io::stderr))
        .init();
}

#[cfg(test)]
#[path = "monitor_mode_tests.rs"]
mod tests;
