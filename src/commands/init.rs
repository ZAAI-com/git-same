//! Init command handler.

use crate::cli::InitArgs;
use crate::config::Config;
use crate::errors::{AppError, Result};
use crate::output::Output;

/// Initialize gisa configuration.
pub async fn run(args: &InitArgs, output: &Output) -> Result<()> {
    let config_path = args.path.clone().unwrap_or_else(Config::default_path);

    // Check if config already exists
    if config_path.exists() && !args.force {
        return Err(AppError::config(format!(
            "Config file already exists at {}. Use --force to overwrite.",
            config_path.display()
        )));
    }

    // Create parent directory
    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| AppError::path(format!("Failed to create config directory: {}", e)))?;
    }

    // Write default config
    let default_config = Config::default_toml();
    std::fs::write(&config_path, default_config)
        .map_err(|e| AppError::path(format!("Failed to write config: {}", e)))?;

    output.success(&format!("Created config at {}", config_path.display()));
    output.info("Edit this file to customize git-same behavior");
    output.info("Run 'git-same clone <path>' to clone your repositories");

    Ok(())
}
