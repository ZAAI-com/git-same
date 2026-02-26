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
