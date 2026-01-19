//! Environment variable token authentication.
//!
//! Retrieves authentication tokens from environment variables.

use crate::errors::AppError;
use std::env;

/// Default environment variable names to check for tokens.
pub const DEFAULT_TOKEN_VARS: &[&str] = &["GITHUB_TOKEN", "GH_TOKEN", "GISA_TOKEN"];

/// Get token from a specific environment variable.
pub fn get_token(var_name: &str) -> Result<String, AppError> {
    env::var(var_name).map_err(|_| {
        AppError::auth(format!(
            "Environment variable {} is not set",
            var_name
        ))
    })
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
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_get_token_missing() {
        let unique_var = "GISA_TEST_NONEXISTENT_VAR_12345";
        env::remove_var(unique_var);

        let result = get_token(unique_var);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not set"));
    }

    #[test]
    fn test_get_token_present() {
        let unique_var = "GISA_TEST_TOKEN_VAR";
        env::set_var(unique_var, "test_token_value");

        let result = get_token(unique_var);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "test_token_value");

        env::remove_var(unique_var);
    }

    #[test]
    fn test_has_token_in_env_false() {
        // Save current values
        let saved: Vec<_> = DEFAULT_TOKEN_VARS
            .iter()
            .map(|v| (v, env::var(v).ok()))
            .collect();

        // Clear all
        for var in DEFAULT_TOKEN_VARS {
            env::remove_var(var);
        }

        assert!(!has_token_in_env());

        // Restore
        for (var, value) in saved {
            if let Some(v) = value {
                env::set_var(var, v);
            }
        }
    }

    #[test]
    fn test_validate_token_format_empty() {
        let result = validate_token_format("");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("empty"));
    }

    #[test]
    fn test_validate_token_format_too_short() {
        let result = validate_token_format("abc");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("short"));
    }

    #[test]
    fn test_validate_token_format_valid_ghp() {
        let result = validate_token_format("ghp_1234567890abcdefghij");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_token_format_valid_gho() {
        let result = validate_token_format("gho_1234567890abcdefghij");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_token_format_valid_fine_grained() {
        let result = validate_token_format("github_pat_1234567890abcdefghij");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_token_format_valid_classic() {
        // Classic tokens are alphanumeric without prefix
        let result = validate_token_format("abcdef1234567890abcdef1234567890abcdef12");
        assert!(result.is_ok());
    }

    #[test]
    fn test_default_token_vars_order() {
        assert_eq!(DEFAULT_TOKEN_VARS[0], "GITHUB_TOKEN");
        assert_eq!(DEFAULT_TOKEN_VARS[1], "GH_TOKEN");
        assert_eq!(DEFAULT_TOKEN_VARS[2], "GISA_TOKEN");
    }
}
