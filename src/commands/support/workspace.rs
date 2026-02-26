use crate::config::{WorkspaceConfig, WorkspaceManager};
use crate::errors::{AppError, Result};
use crate::output::Output;
use std::io::{self, BufRead, Write};

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
        "Update workspace at '{}' to use '{}'? [y/N] ",
        workspace.base_path,
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
