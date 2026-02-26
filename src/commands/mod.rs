//! Command handlers for the CLI subcommands.
//!
//! This module contains the runtime behavior for each subcommand,
//! separated from `main.rs` so the entrypoint stays focused on bootstrapping.

pub mod init;
pub mod reset;
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
use crate::config::Config;
use crate::errors::{AppError, Result};
use crate::output::Output;
use std::path::Path;

pub(crate) use support::{ensure_base_path, warn_if_concurrency_capped};

/// Run the specified command.
pub async fn run_command(
    command: &Command,
    config_path: Option<&Path>,
    output: &Output,
) -> Result<()> {
    // Commands that don't need config
    if let Command::Init(args) = command {
        return run_init(args, output).await;
    }
    if let Command::Reset(args) = command {
        return reset::run(args, output).await;
    }
    #[cfg(feature = "tui")]
    if let Command::Setup(args) = command {
        return setup::run(args, output).await;
    }

    // Load config for all other commands
    let config = load_config(config_path)?;

    match command {
        Command::Init(_) | Command::Reset(_) => unreachable!(),
        #[cfg(feature = "tui")]
        Command::Setup(_) => unreachable!(),
        Command::Sync(args) => run_sync_cmd(args, &config, output).await,
        Command::Status(args) => run_status(args, &config, output).await,
        Command::Workspace(args) => workspace::run(args, &config, output),
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
