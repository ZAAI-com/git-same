//! Startup path for a monitor launched by the managed LaunchAgent.
//!
//! Shared by every host that launchd may exec: the CLI `gisa monitor
//! --foreground --managed` subcommand and, for an app-owned installation,
//! `Git-Same.app/Contents/MacOS/git-same-app monitor --foreground --managed`.
//! Both run the identical loop under the identical rules; only the binary
//! (and therefore the process's TCC identity) differs.

use super::run::{default_shutdown_signal, run_with, Options, RunContext};
use super::runtime_guard::MonitorMode;
use crate::config::Config;
use crate::errors::{AppError, MonitorAgentError, Result};
use crate::ipc::IpcConfig;
use crate::macos::monitor_agent;
use crate::output::Output;
use std::path::PathBuf;
use tracing::error;

/// Runs the monitor as the managed service.
///
/// launchd restarts the program after every unsuccessful exit
/// (`KeepAlive = { SuccessfulExit = false }`). Conditions a restart cannot
/// fix therefore exit successfully after one logged line; only transient
/// failures return an error.
pub async fn run_managed(output: &Output) -> Result<()> {
    let prepared = tokio::task::spawn_blocking(prepare)
        .await
        .map_err(|e| AppError::Other(anyhow::anyhow!("managed startup task failed: {e}")))?;
    let (config, path, ipc_config) = match prepared {
        Ok(prepared) => prepared,
        Err(reason) => {
            error!("{reason}");
            eprintln!("git-same monitor: {reason}");
            return Ok(());
        }
    };

    let opts = Options::from_config(&config, ipc_config, None);
    let context = RunContext {
        mode: MonitorMode::Managed,
        config_path: Some(path),
        interval_explicit: false,
    };
    match run_with(&config, output, opts, context, default_shutdown_signal()).await {
        Err(AppError::MonitorAgent(MonitorAgentError::AlreadyRunning { pid })) => {
            eprintln!("git-same monitor: another monitor is already running ({pid:?}); exiting");
            Ok(())
        }
        other => other,
    }
}

/// Checks that must pass before the first side effect. `Err` is a reason to
/// exit successfully without running.
fn prepare() -> std::result::Result<(Config, PathBuf, IpcConfig), String> {
    let controller = monitor_agent::controller_for_current_user(false)
        .map_err(|e| format!("not starting: {e}"))?;
    match controller.monitoring_enabled() {
        Ok(true) => {}
        Ok(false) => return Err("monitoring is disabled; not starting".to_string()),
        Err(e) => return Err(format!("not starting: {e}")),
    }
    let paths = controller.paths();
    // Never rewritten, never replaced with defaults.
    let config = Config::load_from(&paths.config)
        .map_err(|e| format!("not starting until the configuration is fixed: {e}"))?;
    Ok((config, paths.config.clone(), paths.ipc.clone()))
}
