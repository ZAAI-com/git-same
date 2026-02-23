//! Command handlers for the CLI subcommands.
//!
//! This module contains the runtime behavior for each subcommand,
//! separated from `main.rs` so the entrypoint stays focused on bootstrapping.

pub mod clone;
pub mod init;
#[cfg(feature = "tui")]
pub mod setup;
pub mod status;
pub mod sync;
pub mod sync_cmd;
pub mod workspace;

pub use init::run as run_init;
pub use status::run as run_status;
pub use sync_cmd::run as run_sync_cmd;

use crate::cli::Command;
use crate::config::Config;
use crate::errors::Result;
use crate::operations::clone::MAX_CONCURRENCY;
use crate::operations::sync::SyncMode;
use crate::output::Output;
use std::path::{Path, PathBuf};

/// Run the specified command.
pub async fn run_command(
    command: &Command,
    config_path: Option<&Path>,
    output: &Output,
) -> Result<()> {
    // Init doesn't need config
    if let Command::Init(args) = command {
        return run_init(args, output).await;
    }

    // Setup only needs config for defaults
    #[cfg(feature = "tui")]
    if let Command::Setup(args) = command {
        let config = load_config(config_path)?;
        return setup::run(args, &config, output).await;
    }

    // Load config for all other commands
    let config = load_config(config_path)?;

    match command {
        Command::Init(_) => unreachable!(),
        #[cfg(feature = "tui")]
        Command::Setup(_) => unreachable!(),
        Command::Sync(args) => run_sync_cmd(args, &config, output).await,
        Command::Status(args) => run_status(args, &config, output).await,
        Command::Workspace(args) => workspace::run(args, &config, output),
        Command::Completions(args) => {
            crate::cli::generate_completions(args.shell);
            Ok(())
        }
        // Deprecated commands — show warning then delegate
        Command::Clone(args) => {
            output.warn("'clone' is deprecated. Use 'gisa sync' instead.");
            clone::run(args, &config, output).await
        }
        Command::Fetch(args) => {
            output.warn("'fetch' is deprecated. Use 'gisa sync' instead.");
            sync::run(args, &config, output, SyncMode::Fetch).await
        }
        Command::Pull(args) => {
            output.warn("'pull' is deprecated. Use 'gisa sync --pull' instead.");
            sync::run(args, &config, output, SyncMode::Pull).await
        }
    }
}

/// Load configuration from the given path or default location.
fn load_config(config_path: Option<&Path>) -> Result<Config> {
    if let Some(path) = config_path {
        Config::load_from(path)
    } else {
        Config::load()
    }
}

/// Warn if requested concurrency exceeds the maximum.
/// Returns the effective concurrency to use.
pub(crate) fn warn_if_concurrency_capped(requested: usize, output: &Output) -> usize {
    if requested > MAX_CONCURRENCY {
        output.warn(&format!(
            "Requested concurrency {} exceeds maximum {}. Using {} instead.",
            requested, MAX_CONCURRENCY, MAX_CONCURRENCY
        ));
        MAX_CONCURRENCY
    } else {
        requested
    }
}

/// Expands ~ in a path.
pub(crate) fn expand_path(path: &Path) -> PathBuf {
    let path_str = path.to_string_lossy();
    let expanded = shellexpand::tilde(&path_str);
    PathBuf::from(expanded.as_ref())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::output::{Output, Verbosity};

    fn quiet_output() -> Output {
        Output::new(Verbosity::Quiet, false)
    }

    #[test]
    fn test_concurrency_within_limit() {
        let output = quiet_output();
        assert_eq!(warn_if_concurrency_capped(4, &output), 4);
    }

    #[test]
    fn test_concurrency_at_limit() {
        let output = quiet_output();
        assert_eq!(
            warn_if_concurrency_capped(MAX_CONCURRENCY, &output),
            MAX_CONCURRENCY
        );
    }

    #[test]
    fn test_concurrency_above_limit() {
        let output = quiet_output();
        assert_eq!(
            warn_if_concurrency_capped(MAX_CONCURRENCY + 10, &output),
            MAX_CONCURRENCY
        );
    }

    #[test]
    fn test_expand_path_absolute() {
        let path = Path::new("/tmp/some/path");
        assert_eq!(expand_path(path), PathBuf::from("/tmp/some/path"));
    }

    #[test]
    fn test_expand_path_tilde() {
        let path = Path::new("~/foo");
        let expanded = expand_path(path);
        assert!(!expanded.to_string_lossy().contains('~'));
        assert!(expanded.to_string_lossy().ends_with("/foo"));
    }
}
