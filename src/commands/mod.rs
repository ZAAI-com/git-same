//! Command handlers for the CLI subcommands.
//!
//! This module contains the runtime behavior for each subcommand,
//! separated from `main.rs` so the entrypoint stays focused on bootstrapping.

pub mod clone;
pub mod init;
pub mod reset;
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
use crate::config::{Config, WorkspaceConfig, WorkspaceManager};
use crate::errors::{AppError, Result};
use crate::operations::clone::MAX_CONCURRENCY;
use crate::operations::sync::SyncMode;
use crate::output::Output;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};

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
    if let Command::Completions(args) = command {
        crate::cli::generate_completions(args.shell);
        return Ok(());
    }

    #[cfg(feature = "tui")]
    if let Command::Setup(args) = command {
        return setup::run(args, output).await;
    }

    // Load config for all other commands
    let config = load_config(config_path)?;

    match command {
        Command::Init(_) | Command::Reset(_) | Command::Completions(_) => unreachable!(),
        #[cfg(feature = "tui")]
        Command::Setup(_) => unreachable!(),
        Command::Sync(args) => run_sync_cmd(args, &config, output).await,
        Command::Status(args) => run_status(args, &config, output).await,
        Command::Workspace(args) => workspace::run(args, &config, output),
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

/// Ensure the workspace base_path exists.
///
/// If the configured path is missing, checks whether the current directory
/// could be the new location and offers to update the workspace config.
/// Returns an error if the path cannot be resolved.
pub(crate) fn ensure_base_path(workspace: &mut WorkspaceConfig, output: &Output) -> Result<()> {
    let base_path = workspace.expanded_base_path();
    if base_path.exists() {
        return Ok(());
    }

    let cwd = std::env::current_dir()
        .map_err(|e| AppError::path(format!("Cannot determine current directory: {}", e)))?;

    output.warn(&format!(
        "Base path '{}' does not exist.",
        workspace.base_path
    ));
    output.info(&format!("Current directory: {}", cwd.display()));

    let prompt = format!(
        "Update workspace '{}' to use '{}'? [y/N] ",
        workspace.name,
        cwd.display()
    );

    if confirm_stderr(&prompt)? {
        workspace.base_path = cwd.to_string_lossy().to_string();
        WorkspaceManager::save(workspace)?;
        output.success(&format!("Updated base path to '{}'", workspace.base_path));
        Ok(())
    } else {
        Err(AppError::config(format!(
            "Base path '{}' does not exist. \
             Move to the correct directory and retry, \
             or update manually with 'gisa setup'.",
            base_path.display()
        )))
    }
}

/// Prompt on stderr and return true if the user answers y/yes.
fn confirm_stderr(prompt: &str) -> Result<bool> {
    eprint!("{}", prompt);
    io::stderr().flush()?;

    let stdin = io::stdin();
    let mut line = String::new();
    stdin.lock().read_line(&mut line)?;

    let answer = line.trim().to_lowercase();
    Ok(answer == "y" || answer == "yes")
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
