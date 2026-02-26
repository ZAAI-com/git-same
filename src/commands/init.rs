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

    // Step 3: Next steps
    output.info("Run 'git-same setup' to configure a local folder as workspace.");

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
#[path = "init_tests.rs"]
mod tests;
