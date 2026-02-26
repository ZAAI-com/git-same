use super::*;

#[test]
fn test_from_provider_error() {
    let provider_err = ProviderError::Authentication("bad token".to_string());
    let app_err: AppError = provider_err.into();
    assert!(matches!(app_err, AppError::Provider(_)));
}

#[test]
fn test_from_git_error() {
    let git_err = GitError::GitNotFound;
    let app_err: AppError = git_err.into();
    assert!(matches!(app_err, AppError::Git(_)));
}

#[test]
fn test_from_io_error() {
    let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
    let app_err: AppError = io_err.into();
    assert!(matches!(app_err, AppError::Io(_)));
}

#[test]
fn test_exit_codes_are_distinct() {
    let errors = [
        AppError::Config("test".to_string()),
        AppError::Auth("test".to_string()),
        AppError::Provider(ProviderError::Network("test".to_string())),
        AppError::Git(GitError::GitNotFound),
        AppError::Path("test".to_string()),
        AppError::Cancelled,
    ];

    let codes: Vec<i32> = errors.iter().map(|e| e.exit_code()).collect();
    // Config, Auth, Provider, Git, Path should have unique codes
    assert_eq!(codes[0], 2); // Config
    assert_eq!(codes[1], 3); // Auth
    assert_eq!(codes[2], 4); // Provider
    assert_eq!(codes[3], 5); // Git
    assert_eq!(codes[4], 7); // Path
    assert_eq!(codes[5], 130); // Cancelled
}

#[test]
fn test_is_retryable_delegates_to_inner() {
    let retryable = AppError::Provider(ProviderError::Network("timeout".to_string()));
    assert!(retryable.is_retryable());

    let not_retryable = AppError::Provider(ProviderError::Authentication("bad".to_string()));
    assert!(!not_retryable.is_retryable());
}

#[test]
fn test_config_error_not_retryable() {
    let err = AppError::config("invalid toml");
    assert!(!err.is_retryable());
}

#[test]
fn test_helper_constructors() {
    let err = AppError::config("bad config");
    assert!(matches!(err, AppError::Config(_)));

    let err = AppError::auth("no token");
    assert!(matches!(err, AppError::Auth(_)));

    let err = AppError::path("invalid path");
    assert!(matches!(err, AppError::Path(_)));
}

#[test]
fn test_error_display() {
    let err = AppError::config("missing base_path");
    let display = format!("{}", err);
    assert!(display.contains("Configuration error"));
    assert!(display.contains("missing base_path"));
}

#[test]
fn test_suggested_action_returns_useful_text() {
    let err = AppError::auth("no token found");
    let suggestion = err.suggested_action();
    assert_eq!(
        suggestion,
        "Run 'gh auth login' to authenticate with GitHub CLI"
    );
}
