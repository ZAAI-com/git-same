//! Refresh command handler.
//!
//! User-facing wrapper around the monitor's REFRESH / REFRESH_ALL socket commands.
//! Forces an immediate status.json rewrite so the Finder extension picks up
//! on-disk changes without waiting for the monitor's next poll.

use crate::cli::RefreshArgs;
use git_same_core::config::Config;
use git_same_core::errors::Result;
use git_same_core::output::Output;

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
    use git_same_core::ipc::UnixSocketClient;
    use git_same_core::monitor::runtime_guard;

    let client = UnixSocketClient::new(ipc.socket_path());

    let response = match args.path.as_deref() {
        Some(p) => client.refresh(p).await,
        None => client.refresh_all().await,
    };

    match response {
        Ok(_) => {
            output.success("Monitor refreshed");
            Ok(())
        }
        // The socket is bound only after the first scan. A verified monitor
        // without a socket is starting, not unreachable, and its first scan
        // is the refresh that was asked for.
        Err(_) if runtime_guard::active_monitor(ipc).is_some() => {
            if !output.is_json() {
                println!("Monitor is starting; initial scan in progress. Status will be current when it completes.");
            }
            Ok(())
        }
        Err(e) => Err(git_same_core::errors::AppError::config(format!(
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
