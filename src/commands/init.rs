//! Init command handler.
//!
//! Checks system requirements and writes the global configuration file.

use crate::checks::{self, CheckResult};
use crate::cli::InitArgs;
use crate::config::Config;
use crate::errors::{AppError, Result};
use crate::output::Output;

/// Initialize gisa configuration.
pub async fn run(args: &InitArgs, output: &Output) -> Result<()> {
    // Step 1: Run requirement checks
    output.info("Checking requirements...");
    let results = checks::check_requirements().await;
    display_check_results(&results, output);

    let critical_failures: Vec<&CheckResult> =
        results.iter().filter(|r| !r.passed && r.critical).collect();
    if !critical_failures.is_empty() {
        output.warn("Some critical checks failed. You can still create the config, but gisa may not work correctly.");
    }

    // Step 2: Write global config
    let config_path = match args.path.clone() {
        Some(p) => p,
        None => Config::default_path()?,
    };

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

    // Step 3: Create workspaces directory
    let workspaces_dir = config_path
        .parent()
        .map(|p| p.join("workspaces"))
        .ok_or_else(|| AppError::path("Cannot determine config directory"))?;
    if !workspaces_dir.exists() {
        std::fs::create_dir_all(&workspaces_dir)
            .map_err(|e| AppError::path(format!("Failed to create workspaces directory: {}", e)))?;
    }

    // Step 4: Next steps
    output.info("Run 'gisa setup' to configure a workspace");

    Ok(())
}

/// Display check results with pass/fail indicators.
fn display_check_results(results: &[CheckResult], output: &Output) {
    for result in results {
        if result.passed {
            output.success(&format!("  {} — {}", result.name, result.message));
        } else if result.critical {
            output.error(&format!("  {} — {}", result.name, result.message));
            if let Some(ref suggestion) = result.suggestion {
                output.info(&format!("    {}", suggestion));
            }
        } else {
            output.warn(&format!("  {} — {}", result.name, result.message));
            if let Some(ref suggestion) = result.suggestion {
                output.info(&format!("    {}", suggestion));
            }
        }
    }
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
    async fn test_init_creates_workspaces_dir() {
        let temp = TempDir::new().unwrap();
        let config_path = temp.path().join("git-same/config.toml");
        let args = InitArgs {
            force: false,
            path: Some(config_path.clone()),
        };
        let output = quiet_output();

        let result = run(&args, &output).await;
        assert!(result.is_ok());

        let workspaces_dir = temp.path().join("git-same/workspaces");
        assert!(workspaces_dir.exists());
        assert!(workspaces_dir.is_dir());
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

    #[test]
    fn test_display_check_results_no_panic() {
        let results = vec![
            CheckResult {
                name: "Git".to_string(),
                passed: true,
                message: "git 2.43.0".to_string(),
                suggestion: None,
                critical: true,
            },
            CheckResult {
                name: "SSH".to_string(),
                passed: false,
                message: "no keys".to_string(),
                suggestion: Some("Generate a key".to_string()),
                critical: false,
            },
        ];
        let output = quiet_output();
        display_check_results(&results, &output);
    }
}
