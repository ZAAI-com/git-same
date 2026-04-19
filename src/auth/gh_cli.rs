//! GitHub CLI authentication integration.
//!
//! Uses the `gh` CLI tool to obtain authentication tokens securely.

use crate::errors::AppError;
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

/// Maximum time to wait for any `gh` subprocess to complete.
pub(crate) const GH_COMMAND_TIMEOUT: Duration = Duration::from_secs(10);

/// Run a `gh` subcommand with a hard timeout, killing the child on expiry.
///
/// Prevents the async runtime from being blocked indefinitely if `gh` stalls
/// (e.g. on network issues, interactive prompts, or a wedged SSH agent).
fn run_gh_with_timeout(args: &[&str]) -> Result<Output, AppError> {
    let mut child = Command::new("gh")
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| AppError::auth(format!("Failed to spawn 'gh {}': {}", args.join(" "), e)))?;

    let deadline = Instant::now() + GH_COMMAND_TIMEOUT;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => {
                return child.wait_with_output().map_err(|e| {
                    AppError::auth(format!(
                        "Failed to read output of 'gh {}': {}",
                        args.join(" "),
                        e
                    ))
                });
            }
            Ok(None) => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(AppError::auth(format!(
                        "'gh {}' timed out after {}s",
                        args.join(" "),
                        GH_COMMAND_TIMEOUT.as_secs()
                    )));
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(e) => {
                return Err(AppError::auth(format!(
                    "Failed to wait on 'gh {}': {}",
                    args.join(" "),
                    e
                )));
            }
        }
    }
}

/// Check if the GitHub CLI is installed.
pub fn is_installed() -> bool {
    run_gh_with_timeout(&["--version"])
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Check if the user is authenticated with the GitHub CLI.
pub fn is_authenticated() -> bool {
    run_gh_with_timeout(&["auth", "status"])
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Get the authentication token from the GitHub CLI.
pub fn get_token() -> Result<String, AppError> {
    let output = run_gh_with_timeout(&["auth", "token"])?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AppError::auth(format!(
            "gh auth token failed: {}",
            stderr.trim()
        )));
    }

    let token = String::from_utf8(output.stdout)
        .map_err(|_| AppError::auth("Invalid UTF-8 in token output"))?
        .trim()
        .to_string();

    if token.is_empty() {
        return Err(AppError::auth("gh auth token returned empty token"));
    }

    Ok(token)
}

/// Get the authenticated GitHub username.
pub fn get_username() -> Result<String, AppError> {
    let output = run_gh_with_timeout(&["api", "user", "--jq", ".login"])?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AppError::auth(format!(
            "Failed to get username: {}",
            stderr.trim()
        )));
    }

    let username = String::from_utf8(output.stdout)
        .map_err(|_| AppError::auth("Invalid UTF-8 in username output"))?
        .trim()
        .to_string();

    if username.is_empty() {
        return Err(AppError::auth("gh returned empty username"));
    }

    Ok(username)
}

/// Get token for a specific GitHub host (for GitHub Enterprise).
pub fn get_token_for_host(host: &str) -> Result<String, AppError> {
    let output = run_gh_with_timeout(&["auth", "token", "--hostname", host])?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AppError::auth(format!(
            "gh auth token for {} failed: {}",
            host,
            stderr.trim()
        )));
    }

    let token = String::from_utf8(output.stdout)
        .map_err(|_| AppError::auth("Invalid UTF-8 in token output"))?
        .trim()
        .to_string();

    if token.is_empty() {
        return Err(AppError::auth(format!(
            "gh auth token for {} returned empty token",
            host
        )));
    }

    Ok(token)
}

#[cfg(test)]
#[path = "gh_cli_tests.rs"]
mod tests;
