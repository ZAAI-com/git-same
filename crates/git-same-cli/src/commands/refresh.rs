//! Refresh command handler.
//!
//! User-facing wrapper around the monitor's REFRESH / REFRESH_ALL socket commands.
//! Queues an immediate rescan so the Finder extension picks up on-disk changes
//! without waiting for the monitor's next poll. The monitor answers "OK" once
//! the request is queued and rewrites status.json when the scan completes.

use crate::cli::RefreshArgs;
use git_same_core::config::Config;
use git_same_core::errors::Result;
use git_same_core::output::Output;
#[cfg(unix)]
use std::time::Duration;

/// How long `gisa refresh` waits for the monitor to acknowledge the request.
#[cfg(unix)]
const REFRESH_TIMEOUT: Duration = Duration::from_secs(5);

/// Ask the running monitor to refresh its status cache.
pub async fn run(args: &RefreshArgs, _config: &Config, output: &Output) -> Result<()> {
    run_impl(args, output).await
}

#[cfg(unix)]
async fn run_impl(args: &RefreshArgs, output: &Output) -> Result<()> {
    use git_same_core::ipc::IpcConfig;

    run_with_ipc(args, output, &IpcConfig::default_path()?).await
}

#[cfg(unix)]
async fn run_with_ipc(
    args: &RefreshArgs,
    output: &Output,
    ipc: &git_same_core::ipc::IpcConfig,
) -> Result<()> {
    run_with_timeout(args, output, ipc, REFRESH_TIMEOUT).await
}

#[cfg(unix)]
async fn run_with_timeout(
    args: &RefreshArgs,
    output: &Output,
    ipc: &git_same_core::ipc::IpcConfig,
    timeout: Duration,
) -> Result<()> {
    use git_same_core::errors::AppError;
    use git_same_core::ipc::unix_socket::DaemonCommand;
    use git_same_core::ipc::{Reply, UnixSocketClient};
    use git_same_core::monitor::runtime_guard::{self, RuntimeMonitorState};

    let client = UnixSocketClient::new(ipc.socket_path());

    let command = match args.path.as_deref() {
        Some(p) => DaemonCommand::Refresh(p.to_path_buf()),
        None => DaemonCommand::RefreshAll,
    };

    match client.request(&command.to_string(), timeout).await {
        Ok(Reply::Answered(answer)) if answer.trim() == "OK" => {
            output.success("Monitor refresh requested; badges update when the scan completes");
            Ok(())
        }
        Ok(Reply::Answered(answer)) => {
            let answer = answer.trim();
            let answer = if answer.is_empty() {
                "no answer"
            } else {
                answer
            };
            Err(AppError::config(format!(
                "Monitor rejected the refresh request ({answer})"
            )))
        }
        // An older monitor scans before it answers, and any monitor answers
        // late while a scan runs. The request stays on its socket either way.
        Ok(Reply::Pending) => {
            output.plain(
                "Refresh requested; the monitor is busy and will pick it up when the current scan completes.",
            );
            Ok(())
        }
        // The socket is bound only after the first scan. A verified monitor
        // without a socket is starting, not unreachable, and its first scan
        // is the refresh that was asked for.
        Err(_)
            if matches!(
                runtime_guard::runtime_monitor_state(ipc),
                RuntimeMonitorState::HeldUnknown
            )
                || matches!(
                    runtime_guard::runtime_monitor_state(ipc),
                    RuntimeMonitorState::Active(ref identity)
                        if !runtime_guard::scan_complete(ipc, identity)
                ) =>
        {
            output.plain("Monitor is starting; initial scan in progress. Status will be current when it completes.");
            Ok(())
        }
        Err(e) => Err(AppError::config(format!(
            "Monitor not reachable ({e}). Start it with `gisa monitor --start` or run `gisa monitor`."
        ))),
    }
}

#[cfg(not(unix))]
async fn run_impl(_args: &RefreshArgs, output: &Output) -> Result<()> {
    output.warn("`gisa refresh` is unix-only for now (no monitor socket on this platform).");
    Ok(())
}

#[cfg(test)]
#[path = "refresh_tests.rs"]
mod tests;
