use git_same_core::config::WorkspaceConfig;
use git_same_core::errors::{AppError, Result};
use git_same_core::output::Output;

/// Ensure the workspace root path exists.
///
/// If the configured path is missing, returns an error advising the user
/// to re-run `gisa setup`.
pub(crate) fn ensure_base_path(workspace: &WorkspaceConfig, output: &Output) -> Result<()> {
    let base_path = workspace.expanded_base_path();
    if base_path.is_dir() {
        return Ok(());
    }
    if base_path.exists() {
        return Err(AppError::config(format!(
            "Base path '{}' exists but is not a directory.",
            base_path.display()
        )));
    }

    output.warn(&format!(
        "Base path '{}' does not exist. Run 'gisa setup' to reconfigure.",
        base_path.display()
    ));
    Err(AppError::config(format!(
        "Base path '{}' does not exist.",
        base_path.display()
    )))
}

#[cfg(test)]
#[path = "workspace_tests.rs"]
mod tests;
