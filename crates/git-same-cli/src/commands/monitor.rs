//! `gisa monitor`: start, stop, or query the long-running monitor process.
//!
//! The actual run-loop lives in `git_same_core::monitor`. This file is the
//! CLI surface only: parse args, handle `--status` / `--stop` locally, build
//! the shutdown future from `ctrl_c` + SIGTERM, and call into core.
//!
//! `--status` and `--stop` stay in the CLI because they don't need the loop:
//! they just read the status file or send a kill signal to the recorded PID.

use crate::cli::MonitorArgs;
use git_same_core::config::Config;
use git_same_core::errors::Result;
use git_same_core::ipc::{IpcConfig, StatusFileWriter};
use git_same_core::monitor;
use git_same_core::output::Output;
use std::time::Duration;
use tracing::info;

/// Run the `monitor` subcommand.
pub async fn run(args: &MonitorArgs, config: &Config, output: &Output) -> Result<()> {
    let ipc_config = IpcConfig::default_path()?;
    ipc_config.ensure_dir()?;

    if args.status {
        return show_status(&ipc_config, output);
    }
    if args.stop {
        return stop_monitor(&ipc_config, output);
    }

    info!("Starting git-same monitor");

    let opts = monitor::Options {
        interval: Duration::from_secs(args.interval),
        ipc_config,
    };

    monitor::run(config, output, opts, shutdown_signal()).await
}

/// Resolve when the user hits ctrl-c (SIGINT) or `gisa monitor --stop`
/// sends SIGTERM. Used as the shutdown future for the monitor loop.
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

/// Show monitor status.
///
/// User-facing diagnostic output is printed directly so it is not suppressed
/// by the default Quiet verbosity: `--status` must always answer.
fn show_status(ipc_config: &IpcConfig, _output: &Output) -> Result<()> {
    let status_path = ipc_config.status_file_path();
    if !status_path.exists() {
        println!("Monitor is not running (no status file found)");
        return Ok(());
    }

    let writer = StatusFileWriter::new(status_path);
    match writer.read() {
        Ok(status) => {
            let pid = status.daemon_pid;
            if is_process_alive(pid) {
                println!("Monitor is running (PID: {})", pid);
            } else {
                println!("Monitor is not running (stale PID: {})", pid);
            }
            println!("Last scan: {}", status.timestamp);
            println!("Repos monitored: {}", status.repos.len());
            println!(
                "Workspaces: {}",
                status
                    .workspaces
                    .iter()
                    .map(|w| w.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
        Err(e) => {
            eprintln!("Could not read status file: {}", e);
        }
    }
    Ok(())
}

/// Stop a running monitor process.
fn stop_monitor(ipc_config: &IpcConfig, _output: &Output) -> Result<()> {
    let status_path = ipc_config.status_file_path();
    if !status_path.exists() {
        println!("No monitor is running");
        return Ok(());
    }

    let writer = StatusFileWriter::new(status_path);
    match writer.read() {
        Ok(status) => {
            let pid = status.daemon_pid;
            if is_process_alive(pid) {
                let _ = std::process::Command::new("kill")
                    .args(["-TERM", &pid.to_string()])
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .status();
                println!("Sent stop signal to monitor (PID: {})", pid);
            } else {
                println!("Monitor is not running (stale status file)");
            }
        }
        Err(_) => {
            println!("Could not read monitor status");
        }
    }
    Ok(())
}

/// Check if a process with the given PID is alive via `kill -0`.
fn is_process_alive(pid: u32) -> bool {
    std::process::Command::new("kill")
        .args(["-0", &pid.to_string()])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

#[cfg(test)]
#[path = "monitor_tests.rs"]
mod tests;
