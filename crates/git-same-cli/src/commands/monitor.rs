//! `gisa monitor`: run, control, or inspect the monitor.
//!
//! The run loop lives in `git_same_core::monitor` and the service lifecycle
//! in `git_same_core::macos::monitor_agent`. This file is the CLI surface:
//! pick the mode, adapt output (human text or one JSON object), and build
//! the shutdown future from `ctrl_c` + SIGTERM.
//!
//! Control modes are dispatched before any configuration is loaded: stopping
//! or inspecting the service must work without a valid repository config.

use crate::cli::MonitorArgs;
use git_same_core::config::Config;
use git_same_core::errors::{AppError, MonitorAgentError, Result};
use git_same_core::ipc::{IpcConfig, StatusFileWriter};
use git_same_core::macos::monitor_agent::{self, MonitorAgentState, MonitorAgentStatus};
use git_same_core::monitor::{self, runtime_guard, MonitorMode, RunContext};
use git_same_core::output::Output;
use std::path::Path;
use std::time::Duration;
use tracing::{error, info};

/// Run a control or packaging mode. Needs no configuration.
pub async fn run_control(
    args: &MonitorArgs,
    config_path: Option<&Path>,
    output: &Output,
) -> Result<()> {
    let json = output.is_json();
    let config_override = config_path.is_some();
    let args = ControlArgs::from(args);
    // launchctl and file copies block; keep them off the async runtime.
    let result = tokio::task::spawn_blocking(move || control(&args, config_override))
        .await
        .map_err(|e| AppError::Other(anyhow::anyhow!("monitor control task failed: {e}")))?;
    match result {
        Ok(report) => {
            report.print(json);
            Ok(())
        }
        Err(error) => {
            if json {
                // `--json` prints exactly one object on stdout, failures included.
                println!(
                    "{}",
                    serde_json::json!({ "error": error.to_string(), "exit_code": error.exit_code() })
                );
            }
            Err(error)
        }
    }
}

/// Owned copy of the control flags, movable into a blocking task.
struct ControlArgs {
    agent_protocol_version: bool,
    start: bool,
    stop: bool,
    uninstall: bool,
    install_agent: bool,
    remove_agent: bool,
    app_path: Option<std::path::PathBuf>,
    installer_copy: Option<std::path::PathBuf>,
}

impl From<&MonitorArgs> for ControlArgs {
    fn from(args: &MonitorArgs) -> Self {
        Self {
            agent_protocol_version: args.agent_protocol_version,
            start: args.start,
            stop: args.stop,
            uninstall: args.uninstall,
            install_agent: args.install_agent,
            remove_agent: args.remove_agent,
            app_path: args.app_path.clone(),
            installer_copy: args.installer_copy.clone(),
        }
    }
}

/// What a control mode has to say.
struct Report {
    headline: String,
    status: Option<MonitorAgentStatus>,
    data: Option<DataSummary>,
}

/// Badge-data facts from `status.json`, separate from service state.
struct DataSummary {
    last_written: String,
    repos: usize,
    workspaces: Vec<String>,
}

impl Report {
    fn print(&self, json: bool) {
        if json {
            let value = match &self.status {
                Some(status) => serde_json::to_value(status).unwrap_or_default(),
                None => serde_json::json!({ "message": self.headline }),
            };
            println!("{value}");
            return;
        }
        // Printed directly: the default verbosity is Quiet, and a control
        // command must always answer.
        println!("{}", self.headline);
        if let Some(status) = &self.status {
            print_status_details(status);
        }
        if let Some(data) = &self.data {
            println!("Status file written: {}", data.last_written);
            println!("Repos monitored: {}", data.repos);
            println!("Workspaces: {}", data.workspaces.join(", "));
        }
    }
}

fn print_status_details(status: &MonitorAgentStatus) {
    println!("State: {}", state_name(status.state));
    if let Some(pid) = status.pid {
        let mode = match status.mode {
            Some(MonitorMode::Managed) => " (background service)",
            Some(MonitorMode::Foreground) => " (foreground)",
            None => "",
        };
        println!("PID: {pid}{mode}");
    }
    if status.state != MonitorAgentState::Unsupported {
        println!(
            "Autostart: {}{}",
            if status.autostart { "on" } else { "off" },
            if status.launchd_disabled {
                ", disabled in launchd"
            } else {
                ""
            }
        );
    }
    if let Some(helper) = &status.binary_path {
        let version = status
            .helper_version
            .as_deref()
            .unwrap_or("unknown version");
        println!("Helper: {helper} ({version})");
    }
    if let Some(source) = &status.source {
        println!("Installed from: {source}");
    }
    if let Some(last_scan) = &status.last_scan {
        println!("Last scan: {last_scan}");
    }
    if let Some(detail) = &status.detail {
        println!("Detail: {detail}");
    }
}

fn state_name(state: MonitorAgentState) -> String {
    serde_json::to_value(state)
        .ok()
        .and_then(|value| value.as_str().map(str::to_string))
        .unwrap_or_default()
}

fn control(args: &ControlArgs, config_override: bool) -> Result<Report> {
    // `main.rs` answers this one before logging, the banner, and any config
    // access, which is what makes it side-effect-free for the cask's
    // compatibility probe. The branch is repeated here so the guarantee does
    // not depend on that early return staying where it is: without it, the
    // fallthrough below would run `inspect()` and two `launchctl` calls.
    if args.agent_protocol_version {
        return Ok(Report {
            headline: monitor_agent::PACKAGING_PROTOCOL_VERSION.to_string(),
            status: None,
            data: None,
        });
    }
    if args.install_agent {
        return install_agent(args, config_override);
    }
    if args.remove_agent {
        let controller = monitor_agent::controller_for_current_user(config_override)?;
        let app_path = args.app_path.as_deref().unwrap_or(Path::new(""));
        let removed = controller.remove_for_cask(app_path)?;
        return Ok(Report {
            headline: if removed {
                "Removed the background monitor installed by this cask".to_string()
            } else {
                "No background monitor owned by this cask; nothing removed".to_string()
            },
            status: None,
            data: None,
        });
    }
    if args.start {
        let status = monitor_agent::controller_for_current_user(config_override)?.start()?;
        return Ok(report(status));
    }
    if args.uninstall {
        let status = monitor_agent::controller_for_current_user(config_override)?.uninstall()?;
        return Ok(Report {
            headline:
                "Background monitor removed. Repositories, configuration, and logs were kept."
                    .to_string(),
            status: Some(status),
            data: None,
        });
    }
    if args.stop {
        return stop(config_override);
    }
    status(config_override)
}

fn report(status: MonitorAgentStatus) -> Report {
    Report {
        headline: status.message.clone(),
        status: Some(status),
        data: None,
    }
}

fn install_agent(args: &ControlArgs, config_override: bool) -> Result<Report> {
    let controller = monitor_agent::controller_for_current_user(config_override)?;
    // The staged bundle's CLI is the copy source: the final app path does
    // not exist yet while Homebrew runs the installer.
    let staged = std::env::current_exe()
        .and_then(std::fs::canonicalize)
        .map_err(|e| MonitorAgentError::MissingSource(format!("installer executable: {e}")))?;
    let app_path = args.app_path.as_deref().unwrap_or(Path::new(""));
    let retained = args.installer_copy.as_deref().unwrap_or(Path::new(""));
    let status = controller.install_for_cask(&staged, app_path, retained)?;
    Ok(report(status))
}

/// Persistent Stop on macOS. Elsewhere there is no managed service, so stop
/// the verified running monitor and persist nothing.
fn stop(config_override: bool) -> Result<Report> {
    match monitor_agent::controller_for_current_user(config_override) {
        Ok(controller) => Ok(report(controller.stop()?)),
        Err(MonitorAgentError::Unsupported) => stop_unmanaged(),
        Err(error) => Err(error.into()),
    }
}

fn stop_unmanaged() -> Result<Report> {
    let ipc = IpcConfig::default_path()?;
    // Only a verified lock holder is signalled; a PID from a stale status
    // file never is.
    let headline = match runtime_guard::active_monitor(&ipc) {
        Some(active) => {
            monitor::process::terminate(active.pid)?;
            format!("Sent stop signal to monitor (PID: {})", active.pid)
        }
        None => "No monitor is running".to_string(),
    };
    Ok(Report {
        headline,
        status: None,
        data: None,
    })
}

/// Always informative: works without a configuration or a status file.
fn status(config_override: bool) -> Result<Report> {
    let ipc = IpcConfig::default_path()?;
    let data = read_data_summary(&ipc);
    let status = match monitor_agent::controller_for_current_user(config_override) {
        Ok(controller) => controller.inspect()?,
        // Not the real user's default environment, or not macOS: describe
        // the running process only. Inspecting changes nothing either way.
        Err(_) => unmanaged_status(&ipc),
    };
    Ok(Report {
        headline: status.message.clone(),
        status: Some(status),
        data,
    })
}

fn unmanaged_status(ipc: &IpcConfig) -> MonitorAgentStatus {
    let mut status = MonitorAgentStatus::unsupported();
    match runtime_guard::active_monitor(ipc) {
        Some(active) => {
            let scanned = StatusFileWriter::new(ipc.status_file_path())
                .read()
                .is_ok_and(|file| file.daemon_pid == active.pid);
            status.running = true;
            status.pid = Some(active.pid);
            status.mode = Some(active.mode);
            status.message = if scanned {
                format!("Monitor is running (PID: {})", active.pid)
            } else {
                format!(
                    "Monitor is starting; initial scan in progress (PID: {})",
                    active.pid
                )
            };
        }
        None => status.message = "Monitor is not running".to_string(),
    }
    status
}

fn read_data_summary(ipc: &IpcConfig) -> Option<DataSummary> {
    let path = ipc.status_file_path();
    if !path.exists() {
        return None;
    }
    let file = StatusFileWriter::new(path).read().ok()?;
    Some(DataSummary {
        last_written: file.timestamp,
        repos: file.repos.len(),
        workspaces: file.workspaces.iter().map(|w| w.name.clone()).collect(),
    })
}

/// Run the monitor loop in the foreground, or as the launchd-managed helper.
pub async fn run_foreground(
    args: &MonitorArgs,
    config_path: Option<&Path>,
    output: &Output,
) -> Result<()> {
    if args.managed {
        return run_managed(output).await;
    }

    let path = match config_path {
        Some(path) => path.to_path_buf(),
        None => Config::default_path()?,
    };
    // Defaults when the file is missing: a monitor with no workspaces idles.
    let config = Config::load_from(&path)?;
    let ipc_config = IpcConfig::default_path()?;
    info!("Starting git-same monitor");

    let interval_secs = resolve_interval_secs(args.interval, config.monitor.fullscan_interval_secs);
    let opts = monitor::Options {
        interval: Duration::from_secs(interval_secs),
        ipc_config,
    };
    let context = RunContext {
        mode: MonitorMode::Foreground,
        config_path: Some(path),
        interval_explicit: args.interval.is_some(),
    };
    monitor::run_with(&config, output, opts, context, shutdown_signal()).await
}

/// launchd restarts the helper after every unsuccessful exit
/// (`KeepAlive = { SuccessfulExit = false }`). Conditions that a restart
/// cannot fix therefore exit successfully after one logged line; only
/// transient failures return an error.
async fn run_managed(output: &Output) -> Result<()> {
    let prepared = tokio::task::spawn_blocking(prepare_managed)
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

    let opts = monitor::Options {
        interval: Duration::from_secs(config.monitor.fullscan_interval_secs),
        ipc_config,
    };
    let context = RunContext {
        mode: MonitorMode::Managed,
        config_path: Some(path),
        interval_explicit: false,
    };
    match monitor::run_with(&config, output, opts, context, shutdown_signal()).await {
        Err(AppError::MonitorAgent(MonitorAgentError::AlreadyRunning { pid })) => {
            eprintln!("git-same monitor: another monitor is already running ({pid:?}); exiting");
            Ok(())
        }
        other => other,
    }
}

/// Checks that must pass before the helper's first side effect. `Err` is a
/// reason to exit successfully without running.
fn prepare_managed() -> std::result::Result<(Config, std::path::PathBuf, IpcConfig), String> {
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

/// Resolve the effective polling interval: an explicit `--interval` flag wins,
/// otherwise fall back to the value from `config.toml`.
fn resolve_interval_secs(cli_flag: Option<u64>, config_value: u64) -> u64 {
    cli_flag.unwrap_or(config_value)
}

/// Resolve when the user hits ctrl-c (SIGINT) or a stop request sends
/// SIGTERM. Used as the shutdown future for the monitor loop.
async fn shutdown_signal() {
    #[cfg(unix)]
    {
        let mut sigterm =
            match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
                Ok(s) => s,
                Err(_) => {
                    let _ = tokio::signal::ctrl_c().await;
                    return;
                }
            };
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {},
            _ = sigterm.recv() => {},
        }
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}

#[cfg(test)]
#[path = "monitor_tests.rs"]
mod tests;
