//! GitHub CLI authentication integration.
//!
//! Uses the `gh` CLI tool to obtain authentication tokens securely.

use crate::errors::AppError;
use std::process::Command;

/// Check if the GitHub CLI is installed.
pub fn is_installed() -> bool {
    Command::new("gh")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Check if the user is authenticated with the GitHub CLI.
pub fn is_authenticated() -> bool {
    Command::new("gh")
        .args(["auth", "status"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Get the authentication token from the GitHub CLI.
pub fn get_token() -> Result<String, AppError> {
    let output = Command::new("gh")
        .args(["auth", "token"])
        .output()
        .map_err(|e| AppError::auth(format!("Failed to run 'gh auth token': {}", e)))?;

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
    let output = Command::new("gh")
        .args(["api", "user", "--jq", ".login"])
        .output()
        .map_err(|e| AppError::auth(format!("Failed to get username from gh: {}", e)))?;

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
    let output = Command::new("gh")
        .args(["auth", "token", "--hostname", host])
        .output()
        .map_err(|e| {
            AppError::auth(format!(
                "Failed to run 'gh auth token --hostname {}': {}",
                host, e
            ))
        })?;

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
mod tests {
    use super::*;

    #[test]
    fn test_is_installed_returns_bool() {
        // This test just verifies the function runs without panicking
        // The actual result depends on whether gh is installed
        let _result = is_installed();
    }

    #[test]
    fn test_is_authenticated_returns_bool() {
        let _result = is_authenticated();
    }

    // Integration tests that require gh to be installed and authenticated
    // These are ignored by default
    #[test]
    #[ignore]
    fn test_get_token_when_authenticated() {
        if !is_installed() || !is_authenticated() {
            return;
        }

        let token = get_token().unwrap();
        assert!(!token.is_empty());
        // GitHub tokens start with specific prefixes
        assert!(
            token.starts_with("ghp_")
                || token.starts_with("gho_")
                || token.starts_with("ghu_")
                || token.starts_with("ghr_")
                || token.starts_with("ghs_")
        );
    }

    #[test]
    #[ignore]
    fn test_get_username_when_authenticated() {
        if !is_installed() || !is_authenticated() {
            return;
        }

        let username = get_username().unwrap();
        assert!(!username.is_empty());
        // Usernames shouldn't contain whitespace
        assert!(!username.contains(char::is_whitespace));
    }
}
