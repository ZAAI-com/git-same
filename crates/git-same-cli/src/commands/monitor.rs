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

    let interval_secs = resolve_interval_secs(args.interval, config.monitor.fullscan_interval_secs);
    let opts = monitor::Options {
        interval: Duration::from_secs(interval_secs),
        ipc_config,
    };

    monitor::run(config, output, opts, shutdown_signal()).await
}

/// Resolve the effective polling interval: an explicit `--interval` flag wins,
/// otherwise fall back to the value from `config.toml`.
fn resolve_interval_secs(cli_flag: Option<u64>, config_value: u64) -> u64 {
    cli_flag.unwrap_or(config_value)
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
#[cfg(unix)]
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

/// Non-Unix fallback: the monitor relies on Unix sockets and POSIX signals,
/// so there is no running process to stop on these platforms.
#[cfg(not(unix))]
fn stop_monitor(_ipc_config: &IpcConfig, _output: &Output) -> Result<()> {
    println!("Monitor stop is not supported on this platform");
    Ok(())
}

/// Check if a process with the given PID is alive via `kill -0`.
#[cfg(unix)]
fn is_process_alive(pid: u32) -> bool {
    // A live process has a positive PID that fits in pid_t (i32). Reject 0 and
    // anything above i32::MAX up front: u32::MAX would reach `kill` as -1 and be
    // treated as a process-group/broadcast target, which returns success on
    // Linux and would falsely report the process alive.
    if pid == 0 || pid > i32::MAX as u32 {
        return false;
    }
    std::process::Command::new("kill")
        .args(["-0", &pid.to_string()])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Non-Unix fallback: without POSIX signals we can't probe liveness, so assume
/// the recorded PID is alive rather than report a false negative. PIDs that
/// cannot be valid (0 or above i32::MAX, e.g. the u32::MAX staleness sentinel)
/// are still reported as dead.
#[cfg(not(unix))]
fn is_process_alive(pid: u32) -> bool {
    pid != 0 && pid <= i32::MAX as u32
}

#[cfg(test)]
#[path = "monitor_tests.rs"]
mod tests;
