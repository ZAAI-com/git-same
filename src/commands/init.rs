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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::InitArgs;
    use tempfile::TempDir;

    fn quiet_output() -> Output {
        Output::new(crate::output::Verbosity::Quiet, false)
    }

    #[tokio::test]
    async fn test_init_creates_config() {
        let temp = TempDir::new().unwrap();
        let config_path = temp.path().join("config.toml");
        let args = InitArgs {
            force: false,
            path: Some(config_path.clone()),
        };
        let output = quiet_output();

        let result = run(&args, &output).await;
        assert!(result.is_ok());
        assert!(config_path.exists());
        let content = std::fs::read_to_string(&config_path).unwrap();
        assert!(!content.is_empty());
    }

    #[tokio::test]
    async fn test_init_fails_if_exists_without_force() {
        let temp = TempDir::new().unwrap();
        let config_path = temp.path().join("config.toml");
        std::fs::write(&config_path, "existing").unwrap();

        let args = InitArgs {
            force: false,
            path: Some(config_path),
        };
        let output = quiet_output();

        let result = run(&args, &output).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_init_overwrites_with_force() {
        let temp = TempDir::new().unwrap();
        let config_path = temp.path().join("config.toml");
        std::fs::write(&config_path, "old content").unwrap();

        let args = InitArgs {
            force: true,
            path: Some(config_path.clone()),
        };
        let output = quiet_output();

        let result = run(&args, &output).await;
        assert!(result.is_ok());
        let content = std::fs::read_to_string(&config_path).unwrap();
        assert_ne!(content, "old content");
    }

    #[tokio::test]
    async fn test_init_creates_parent_dirs() {
        let temp = TempDir::new().unwrap();
        let config_path = temp.path().join("nested/deep/config.toml");
        let args = InitArgs {
            force: false,
            path: Some(config_path.clone()),
        };
        let output = quiet_output();

        let result = run(&args, &output).await;
        assert!(result.is_ok());
        assert!(config_path.exists());
    }
}
