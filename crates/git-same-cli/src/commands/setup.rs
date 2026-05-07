//! Setup command handler.
//!
//! Thin wrapper that launches the interactive setup wizard.

#[cfg(feature = "tui")]
use crate::cli::SetupArgs;
#[cfg(feature = "tui")]
use git_same_core::errors::Result;
#[cfg(feature = "tui")]
use git_same_core::output::Output;

/// Run the setup wizard.
#[cfg(feature = "tui")]
pub async fn run(_args: &SetupArgs, output: &Output) -> Result<()> {
    let completed = crate::setup::run_setup().await?;
    if completed {
        output.success("Workspace configured successfully");
        output.info("Run 'gisa sync' to sync your repositories");
    } else {
        output.info("Setup cancelled");
    }
    Ok(())
}
