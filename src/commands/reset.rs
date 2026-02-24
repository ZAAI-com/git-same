//! Reset command handler.
//!
//! Removes all gisa configuration, workspace configs, and cache,
//! returning the tool to an uninitialized state.

use crate::cache::CacheManager;
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
    workspaces_dir: Option<PathBuf>,
    cache_file: Option<PathBuf>,
}

impl ResetTarget {
    fn is_empty(&self) -> bool {
        self.config_file.is_none() && self.workspaces_dir.is_none() && self.cache_file.is_none()
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

    // Delete in order: cache → workspaces → config → parent dir
    if let Some(ref path) = target.cache_file {
        match std::fs::remove_file(path) {
            Ok(()) => output.success(&format!("Removed cache: {}", path.display())),
            Err(e) => output.warn(&format!("Failed to remove cache: {}", e)),
        }
    }

    if let Some(ref dir) = target.workspaces_dir {
        match std::fs::remove_dir_all(dir) {
            Ok(()) => output.success(&format!(
                "Removed {} workspace config(s)",
                target.workspace_names.len()
            )),
            Err(e) => output.warn(&format!("Failed to remove workspaces directory: {}", e)),
        }
    }

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

    let (workspaces_dir, workspace_names) = match WorkspaceManager::workspaces_dir() {
        Ok(dir) if dir.exists() => {
            let names: Vec<String> = WorkspaceManager::list()
                .unwrap_or_default()
                .iter()
                .map(|ws| ws.name.clone())
                .collect();
            (Some(dir), names)
        }
        _ => (None, Vec::new()),
    };

    let cache_file = CacheManager::default_cache_path()
        .ok()
        .filter(|p| p.exists());

    Ok(ResetTarget {
        config_dir,
        config_file,
        workspace_names,
        workspaces_dir,
        cache_file,
    })
}

/// Display the targets that will be removed.
fn display_targets(target: &ResetTarget, output: &Output) {
    output.warn("The following will be permanently deleted:");

    if let Some(ref path) = target.config_file {
        output.info(&format!("  Global config:  {}", path.display()));
    }

    if let Some(ref dir) = target.workspaces_dir {
        if target.workspace_names.is_empty() {
            output.info(&format!("  Workspaces dir: {} (empty)", dir.display()));
        } else {
            output.info(&format!(
                "  Workspaces ({}): {}",
                target.workspace_names.len(),
                dir.display()
            ));
            for name in &target.workspace_names {
                output.info(&format!("    - {}", name));
            }
        }
    }

    if let Some(ref path) = target.cache_file {
        output.info(&format!("  Cache:          {}", path.display()));
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
            workspaces_dir: None,
            cache_file: None,
        };
        assert!(target.is_empty());
    }

    #[test]
    fn test_reset_target_not_empty_with_config() {
        let target = ResetTarget {
            config_dir: PathBuf::from("/some/dir"),
            config_file: Some(PathBuf::from("/some/dir/config.toml")),
            workspace_names: Vec::new(),
            workspaces_dir: None,
            cache_file: None,
        };
        assert!(!target.is_empty());
    }

    #[test]
    fn test_reset_target_not_empty_with_workspaces() {
        let target = ResetTarget {
            config_dir: PathBuf::from("/some/dir"),
            config_file: None,
            workspace_names: vec!["ws1".to_string()],
            workspaces_dir: Some(PathBuf::from("/some/dir/workspaces")),
            cache_file: None,
        };
        assert!(!target.is_empty());
    }

    #[test]
    fn test_reset_target_not_empty_with_cache() {
        let target = ResetTarget {
            config_dir: PathBuf::from("/some/dir"),
            config_file: None,
            workspace_names: Vec::new(),
            workspaces_dir: None,
            cache_file: Some(PathBuf::from("/some/dir/cache.json")),
        };
        assert!(!target.is_empty());
    }

    #[test]
    fn test_display_targets_no_panic() {
        let target = ResetTarget {
            config_dir: PathBuf::from("/tmp/test"),
            config_file: Some(PathBuf::from("/tmp/test/config.toml")),
            workspace_names: vec!["ws1".to_string(), "ws2".to_string()],
            workspaces_dir: Some(PathBuf::from("/tmp/test/workspaces")),
            cache_file: Some(PathBuf::from("/tmp/test/cache.json")),
        };
        let output = Output::new(crate::output::Verbosity::Quiet, false);
        display_targets(&target, &output);
    }
}
