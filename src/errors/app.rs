//! Application-level error types.
//!
//! These errors represent top-level failures in the gisa application,
//! aggregating errors from providers, git operations, and configuration.

use super::{GitError, ProviderError};
use thiserror::Error;

/// Top-level application errors.
///
/// This enum aggregates all error types that can occur in the application,
/// providing a unified error type for the CLI interface.
#[derive(Error, Debug)]
pub enum AppError {
    /// Configuration file error.
    #[error("Configuration error: {0}")]
    Config(String),

    /// Authentication failed across all methods.
    #[error("Authentication failed: {0}")]
    Auth(String),

    /// Error from a Git hosting provider.
    #[error("Provider error: {0}")]
    Provider(#[from] ProviderError),

    /// Error during a git operation.
    #[error("Git error: {0}")]
    Git(#[from] GitError),

    /// File system I/O error.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Path-related error (invalid path, not found, etc.).
    #[error("Path error: {0}")]
    Path(String),

    /// User cancelled the operation.
    #[error("Operation cancelled by user")]
    Cancelled,

    /// Operation interrupted by signal.
    #[error("Operation interrupted")]
    Interrupted,

    /// Generic error with context.
    #[error("{0}")]
    Other(#[from] anyhow::Error),
}

impl AppError {
    /// Creates a configuration error.
    pub fn config(message: impl Into<String>) -> Self {
        AppError::Config(message.into())
    }

    /// Creates an authentication error.
    pub fn auth(message: impl Into<String>) -> Self {
        AppError::Auth(message.into())
    }

    /// Creates a path error.
    pub fn path(message: impl Into<String>) -> Self {
        AppError::Path(message.into())
    }

    /// Returns `true` if this error is recoverable with a retry.
    pub fn is_retryable(&self) -> bool {
        match self {
            AppError::Provider(e) => e.is_retryable(),
            AppError::Git(e) => e.is_retryable(),
            AppError::Io(e) => {
                // Some I/O errors are retryable
                matches!(
                    e.kind(),
                    std::io::ErrorKind::TimedOut
                        | std::io::ErrorKind::Interrupted
                        | std::io::ErrorKind::WouldBlock
                )
            }
            _ => false,
        }
    }

    /// Returns a user-friendly exit code for this error.
    pub fn exit_code(&self) -> i32 {
        match self {
            AppError::Config(_) => 2,
            AppError::Auth(_) => 3,
            AppError::Provider(_) => 4,
            AppError::Git(_) => 5,
            AppError::Io(_) => 6,
            AppError::Path(_) => 7,
            AppError::Cancelled => 130, // Standard for SIGINT
            AppError::Interrupted => 130,
            AppError::Other(_) => 1,
        }
    }

    /// Returns a suggested action to resolve this error.
    pub fn suggested_action(&self) -> &str {
        match self {
            AppError::Config(_) => "Check your gisa.config.toml file for syntax errors",
            AppError::Auth(_) => "Run 'gh auth login' or set GITHUB_TOKEN environment variable",
            AppError::Provider(e) => e.suggested_action(),
            AppError::Git(e) => e.suggested_action(),
            AppError::Io(_) => "Check file permissions and disk space",
            AppError::Path(_) => "Check that the path exists and is accessible",
            AppError::Cancelled | AppError::Interrupted => "Re-run the command to continue",
            AppError::Other(_) => "Check the error message and try again",
        }
    }
}

/// A convenience type alias for Results in this application.
pub type Result<T> = std::result::Result<T, AppError>;

#[cfg(test)]
mod tests {
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
        assert!(suggestion.contains("gh auth login") || suggestion.contains("GITHUB_TOKEN"));
    }
}
