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
    use git_same_core::ipc::{IpcConfig, UnixSocketClient};

    let cfg = IpcConfig::default_path()?;
    let client = UnixSocketClient::new(cfg.socket_path());

    let response = match args.path.as_deref() {
        Some(p) => client.refresh(p).await,
        None => client.refresh_all().await,
    };

    match response {
        Ok(_) => {
            output.success("Monitor refreshed");
            Ok(())
        }
        Err(e) => {
            output.error("Monitor not reachable. Start it with `gisa monitor`.");
            Err(e)
        }
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
