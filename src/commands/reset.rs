//! Reset command handler.
//!
//! Removes all gisa configuration, workspace configs, and cache,
//! returning the tool to an uninitialized state.

use crate::cli::ResetArgs;
use crate::config::{Config, WorkspaceManager};
use crate::errors::{AppError, Result};
use crate::output::Output;
use std::io::{self, BufRead, Write};
use std::path::PathBuf;

/// What will be removed during reset.
struct ResetTarget {
    config_dir: PathBuf,
    config_file: Option<PathBuf>,
    workspace_names: Vec<String>,
}

impl ResetTarget {
    fn is_empty(&self) -> bool {
        self.config_file.is_none() && self.workspace_names.is_empty()
    }
}

/// Run the reset command.
pub async fn run(args: &ResetArgs, output: &Output) -> Result<()> {
    let target = discover_targets()?;

    if target.is_empty() {
        output.info("Nothing to reset — gisa is not configured.");
        return Ok(());
    }

    display_targets(&target, output);

    if !args.force && !confirm("Are you sure you want to delete all gisa configuration? [y/N] ")? {
        output.info("Reset cancelled.");
        return Ok(());
    }

    // Delete workspaces (each is a subdirectory of config_dir)
    for name in &target.workspace_names {
        match WorkspaceManager::delete(name) {
            Ok(()) => output.success(&format!("Removed workspace: {}", name)),
            Err(e) => output.warn(&format!("Failed to remove workspace '{}': {}", name, e)),
        }
    }

    // Delete global config file
    if let Some(ref path) = target.config_file {
        match std::fs::remove_file(path) {
            Ok(()) => output.success(&format!("Removed config: {}", path.display())),
            Err(e) => output.warn(&format!("Failed to remove config file: {}", e)),
        }
    }

    // Remove config directory if now empty
    if target.config_dir.exists() {
        match std::fs::remove_dir(&target.config_dir) {
            Ok(()) => output.verbose(&format!(
                "Removed directory: {}",
                target.config_dir.display()
            )),
            Err(_) => {
                output.verbose(&format!(
                    "Config directory not empty, leaving: {}",
                    target.config_dir.display()
                ));
            }
        }
    }

    output.success("Reset complete. Run 'gisa init' to start fresh.");
    Ok(())
}

/// Discover what files and directories exist that would be removed.
fn discover_targets() -> Result<ResetTarget> {
    let config_path = Config::default_path()?;
    let config_dir = config_path
        .parent()
        .ok_or_else(|| AppError::config("Cannot determine config directory"))?
        .to_path_buf();

    let config_file = if config_path.exists() {
        Some(config_path)
    } else {
        None
    };

    let workspace_names: Vec<String> = WorkspaceManager::list()
        .unwrap_or_default()
        .iter()
        .map(|ws| ws.name.clone())
        .collect();

    Ok(ResetTarget {
        config_dir,
        config_file,
        workspace_names,
    })
}

/// Display the targets that will be removed.
fn display_targets(target: &ResetTarget, output: &Output) {
    output.warn("The following will be permanently deleted:");

    if let Some(ref path) = target.config_file {
        output.info(&format!("  Global config: {}", path.display()));
    }

    if !target.workspace_names.is_empty() {
        output.info(&format!(
            "  Workspaces ({}, including caches):",
            target.workspace_names.len(),
        ));
        for name in &target.workspace_names {
            output.info(&format!("    - {}", name));
        }
    }
}

/// Prompt the user for confirmation. Returns true if they answer y/yes.
fn confirm(prompt: &str) -> Result<bool> {
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

    #[test]
    fn test_reset_target_is_empty_when_nothing_exists() {
        let target = ResetTarget {
            config_dir: PathBuf::from("/nonexistent"),
            config_file: None,
            workspace_names: Vec::new(),
        };
        assert!(target.is_empty());
    }

    #[test]
    fn test_reset_target_not_empty_with_config() {
        let target = ResetTarget {
            config_dir: PathBuf::from("/some/dir"),
            config_file: Some(PathBuf::from("/some/dir/config.toml")),
            workspace_names: Vec::new(),
        };
        assert!(!target.is_empty());
    }

    #[test]
    fn test_reset_target_not_empty_with_workspaces() {
        let target = ResetTarget {
            config_dir: PathBuf::from("/some/dir"),
            config_file: None,
            workspace_names: vec!["ws1".to_string()],
        };
        assert!(!target.is_empty());
    }

    #[test]
    fn test_display_targets_no_panic() {
        let target = ResetTarget {
            config_dir: PathBuf::from("/tmp/test"),
            config_file: Some(PathBuf::from("/tmp/test/config.toml")),
            workspace_names: vec!["ws1".to_string(), "ws2".to_string()],
        };
        let output = Output::new(crate::output::Verbosity::Quiet, false);
        display_targets(&target, &output);
    }
}
