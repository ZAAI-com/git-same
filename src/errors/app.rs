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
    Provider(
        #[from]
        #[source]
        ProviderError,
    ),

    /// Error during a git operation.
    #[error("Git error: {0}")]
    Git(
        #[from]
        #[source]
        GitError,
    ),

    /// File system I/O error.
    #[error("IO error: {0}")]
    Io(
        #[from]
        #[source]
        std::io::Error,
    ),

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
    Other(
        #[from]
        #[source]
        anyhow::Error,
    ),
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
            AppError::Config(_) => {
                "Check your config file for syntax errors, or run 'gisa init' to create one"
            }
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
#[path = "app_tests.rs"]
mod tests;
