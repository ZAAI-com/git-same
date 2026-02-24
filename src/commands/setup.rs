//! Setup command handler.
//!
//! Thin wrapper that launches the interactive setup wizard.

#[cfg(feature = "tui")]
use crate::cli::SetupArgs;
#[cfg(feature = "tui")]
use crate::errors::Result;
#[cfg(feature = "tui")]
use crate::output::Output;

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
