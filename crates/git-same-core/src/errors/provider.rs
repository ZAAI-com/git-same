//! Provider-specific error types for Git hosting services.
//!
//! These errors represent failures that occur when interacting with
//! provider APIs like GitHub, GitLab, or Bitbucket.

use thiserror::Error;

/// Errors that occur when interacting with a Git hosting provider's API.
#[derive(Error, Debug)]
pub enum ProviderError {
    /// Authentication failed - invalid or expired token.
    #[error("Authentication failed: {0}")]
    Authentication(String),

    /// Network-level error - connection failed, timeout, etc.
    #[error("Network error: {0}")]
    Network(String),

    /// API returned an error response.
    #[error("API error (HTTP {status}): {message}")]
    Api {
        /// HTTP status code
        status: u16,
        /// Error message from the API
        message: String,
    },

    /// Rate limit exceeded.
    #[error("Rate limited. Resets at {reset_time}")]
    RateLimited {
        /// When the rate limit resets (ISO 8601 format)
        reset_time: String,
    },

    /// Failed to parse API response.
    #[error("Failed to parse response: {0}")]
    Parse(String),

    /// Configuration error for the provider.
    #[error("Configuration error: {0}")]
    Configuration(String),

    /// Feature not yet implemented.
    #[error("Not implemented: {0}")]
    NotImplemented(String),

    /// Resource not found (404).
    #[error("Not found: {0}")]
    NotFound(String),

    /// Permission denied (403 without rate limit).
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
}

impl ProviderError {
    /// Returns `true` if this error is potentially recoverable with a retry.
    ///
    /// Retryable errors include:
    /// - Network errors (transient connectivity issues)
    /// - Rate limiting (will succeed after waiting)
    /// - HTTP 429 responses
    /// - Server errors (5xx status codes)
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            ProviderError::Network(_)
                | ProviderError::RateLimited { .. }
                | ProviderError::Api { status: 429, .. }
                | ProviderError::Api {
                    status: 500..=599,
                    ..
                }
        )
    }

    /// Returns a user-friendly suggestion for how to resolve this error.
    pub fn suggested_action(&self) -> &'static str {
        match self {
            ProviderError::Authentication(_) => {
                "Re-authenticate with your Git provider or verify your access token/credentials"
            }
            ProviderError::RateLimited { .. } => {
                "Wait for the rate limit to reset, or use a different authentication token"
            }
            ProviderError::Network(_) => "Check your internet connection and try again",
            ProviderError::Api { status: 403, .. } => {
                "Check that your token has the required permissions for this operation"
            }
            ProviderError::Api { status: 404, .. } | ProviderError::NotFound(_) => {
                "The resource may have been deleted or you may have lost access"
            }
            ProviderError::PermissionDenied(_) => {
                "Check that your token has the required permissions for this operation"
            }
            ProviderError::Configuration(_) => "Check your gisa.config.toml configuration file",
            ProviderError::NotImplemented(_) => {
                "This feature is not yet available. Check for updates"
            }
            _ => "Please check the error message and try again",
        }
    }

    /// Creates an API error from an HTTP status code and message.
    pub fn from_status(status: u16, message: impl Into<String>) -> Self {
        let message = message.into();
        match status {
            401 => ProviderError::Authentication(message),
            403 => ProviderError::PermissionDenied(message),
            404 => ProviderError::NotFound(message),
            _ => ProviderError::Api { status, message },
        }
    }
}

#[cfg(test)]
#[path = "provider_tests.rs"]
mod tests;
