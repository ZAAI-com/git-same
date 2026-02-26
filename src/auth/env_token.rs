//! Environment variable token authentication.
//!
//! Retrieves authentication tokens from environment variables.

use crate::errors::AppError;
use std::env;

/// Default environment variable names to check for tokens.
pub const DEFAULT_TOKEN_VARS: &[&str] = &["GITHUB_TOKEN", "GH_TOKEN", "GISA_TOKEN"];

/// Get token from a specific environment variable.
pub fn get_token(var_name: &str) -> Result<String, AppError> {
    env::var(var_name)
        .map_err(|_| AppError::auth(format!("Environment variable {} is not set", var_name)))
}

/// Get token from any of the default environment variables.
///
/// Checks in order: GITHUB_TOKEN, GH_TOKEN, GISA_TOKEN
pub fn get_token_from_defaults() -> Result<(String, &'static str), AppError> {
    for var_name in DEFAULT_TOKEN_VARS {
        if let Ok(token) = env::var(var_name) {
            if !token.is_empty() {
                return Ok((token, var_name));
            }
        }
    }

    Err(AppError::auth(format!(
        "No token found in environment variables: {}",
        DEFAULT_TOKEN_VARS.join(", ")
    )))
}

/// Check if any of the default token environment variables are set.
pub fn has_token_in_env() -> bool {
    DEFAULT_TOKEN_VARS
        .iter()
        .any(|var| env::var(var).map(|v| !v.is_empty()).unwrap_or(false))
}

/// Validate that a token looks like a valid GitHub token.
///
/// This is a basic format check, not a verification against GitHub's API.
pub fn validate_token_format(token: &str) -> Result<(), String> {
    if token.is_empty() {
        return Err("Token is empty".to_string());
    }

    if token.len() < 10 {
        return Err("Token is too short".to_string());
    }

    // GitHub tokens have specific prefixes
    let valid_prefixes = ["ghp_", "gho_", "ghu_", "ghr_", "ghs_", "github_pat_"];

    // Classic tokens don't have prefixes, so we allow those too
    // Fine-grained tokens start with github_pat_
    let has_known_prefix = valid_prefixes.iter().any(|p| token.starts_with(p));
    let is_classic_token = token.chars().all(|c| c.is_ascii_alphanumeric());

    if !has_known_prefix && !is_classic_token {
        return Err("Token has invalid format".to_string());
    }

    Ok(())
}

#[cfg(test)]
#[path = "env_token_tests.rs"]
mod tests;
