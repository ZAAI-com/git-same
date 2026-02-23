//! Git-Same - Mirror GitHub org/repo structure locally
//!
//! Main entry point for the git-same CLI application.

use git_same::cli::Cli;
use git_same::commands::run_command;
use git_same::output::{Output, Verbosity};
use std::process::ExitCode;
use tracing::debug;

/// Initialize structured logging based on GISA_LOG environment variable.
///
/// Examples:
/// - `GISA_LOG=debug` - Enable debug logging for all modules
/// - `GISA_LOG=git_same=debug` - Enable debug logging for git-same only
/// - `GISA_LOG=git_same::auth=trace` - Enable trace logging for auth module
/// - `GISA_LOG=warn` - Only show warnings and errors
fn init_logging() {
    use tracing_subscriber::{fmt, prelude::*, EnvFilter};

    // Use GISA_LOG env var, defaulting to "warn" if not set
    let filter = EnvFilter::try_from_env("GISA_LOG").unwrap_or_else(|_| EnvFilter::new("warn"));

    tracing_subscriber::registry()
        .with(fmt::layer().with_target(true).with_level(true).compact())
        .with(filter)
        .init();
}

#[tokio::main]
async fn main() -> ExitCode {
    // Initialize logging early
    init_logging();

    let cli = Cli::parse_args();
    debug!(command = ?cli.command, "Parsed CLI arguments");

    match cli.command {
        Some(ref command) => {
            // CLI subcommand mode — existing behavior
            let verbosity = Verbosity::from(cli.verbosity());
            let output = Output::new(verbosity, cli.is_json());

            // Print banner unless quiet or JSON output
            if !output.is_json() && !cli.is_quiet() {
                git_same::banner::print_banner();
            }

            let result = run_command(command, cli.config.as_deref(), &output).await;

            match result {
                Ok(()) => ExitCode::SUCCESS,
                Err(e) => {
                    output.error(&e.to_string());
                    if verbosity >= Verbosity::Verbose {
                        eprintln!("  Suggestion: {}", e.suggested_action());
                    }
                    ExitCode::from(e.exit_code().clamp(1, 255) as u8)
                }
            }
        }
        None => {
            // No subcommand — launch TUI
            #[cfg(feature = "tui")]
            {
                use git_same::config::Config;

                let config = match cli.config.as_ref() {
                    Some(path) => Config::load_from(path),
                    None => Config::load(),
                };

                match config {
                    Ok(config) => match git_same::tui::run_tui(config).await {
                        Ok(()) => ExitCode::SUCCESS,
                        Err(e) => {
                            eprintln!("TUI error: {}", e);
                            ExitCode::from(1)
                        }
                    },
                    Err(e) => {
                        eprintln!("Failed to load config: {}", e);
                        eprintln!("Run 'gisa init' to create a configuration file.");
                        ExitCode::from(2)
                    }
                }
            }
            #[cfg(not(feature = "tui"))]
            {
                eprintln!("TUI not available. Run a subcommand (e.g., 'gisa clone') or build with --features tui.");
                ExitCode::from(1)
            }
        }
    }
}
