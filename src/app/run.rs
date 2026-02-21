//! Command dispatch for the CLI runtime.

use crate::cli::{Cli, Command};
use crate::commands::{run_clone, run_init, run_status, run_sync};
use crate::config::Config;
use crate::errors::Result;
use crate::output::Output;
use crate::sync::SyncMode;

/// Run the specified command.
pub async fn run_command(cli: &Cli, output: &Output) -> Result<()> {
    // Load config
    let config = if let Some(ref path) = cli.config {
        Config::load_from(path)?
    } else {
        Config::load()?
    };

    match &cli.command {
        Command::Init(args) => run_init(args, output).await,
        Command::Clone(args) => run_clone(args, &config, output).await,
        Command::Fetch(args) => run_sync(args, &config, output, SyncMode::Fetch).await,
        Command::Pull(args) => run_sync(args, &config, output, SyncMode::Pull).await,
        Command::Status(args) => run_status(args, &config, output).await,
        Command::Completions(args) => {
            crate::cli::generate_completions(args.shell);
            Ok(())
        }
    }
}
