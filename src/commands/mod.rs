//! Command handlers for the CLI subcommands.
//!
//! This module contains the runtime behavior for each subcommand,
//! separated from `main.rs` so the entrypoint stays focused on bootstrapping.

pub mod clone;
pub mod init;
pub mod status;
pub mod sync;

pub use clone::run as run_clone;
pub use init::run as run_init;
pub use status::run as run_status;
pub use sync::run as run_sync;

use crate::cli::{Cli, Command};
use crate::config::Config;
use crate::errors::Result;
use crate::operations::clone::MAX_CONCURRENCY;
use crate::operations::sync::SyncMode;
use crate::output::Output;
use std::path::{Path, PathBuf};

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
