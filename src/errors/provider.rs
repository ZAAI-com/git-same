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
    /// - Server errors (5xx status codes)
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            ProviderError::Network(_)
                | ProviderError::RateLimited { .. }
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
                "Run 'gh auth login' to re-authenticate, or check your GITHUB_TOKEN"
            }
            ProviderError::RateLimited { .. } => {
                "Wait for the rate limit to reset, or use a different authentication token"
            }
            ProviderError::Network(_) => "Check your internet connection and try again",
            ProviderError::Api { status: 403, .. } => {
                "Check that your token has the required scopes (repo, read:org)"
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
mod tests {
    use super::*;

    #[test]
    fn test_network_error_is_retryable() {
        let err = ProviderError::Network("connection refused".to_string());
        assert!(err.is_retryable());
    }

    #[test]
    fn test_rate_limited_is_retryable() {
        let err = ProviderError::RateLimited {
            reset_time: "2024-01-01T00:00:00Z".to_string(),
        };
        assert!(err.is_retryable());
    }

    #[test]
    fn test_server_error_is_retryable() {
        let err = ProviderError::Api {
            status: 500,
            message: "Internal Server Error".to_string(),
        };
        assert!(err.is_retryable());

        let err = ProviderError::Api {
            status: 503,
            message: "Service Unavailable".to_string(),
        };
        assert!(err.is_retryable());
    }

    #[test]
    fn test_auth_error_is_not_retryable() {
        let err = ProviderError::Authentication("bad token".to_string());
        assert!(!err.is_retryable());
    }

    #[test]
    fn test_client_error_is_not_retryable() {
        let err = ProviderError::Api {
            status: 400,
            message: "Bad Request".to_string(),
        };
        assert!(!err.is_retryable());

        let err = ProviderError::Api {
            status: 404,
            message: "Not Found".to_string(),
        };
        assert!(!err.is_retryable());
    }

    #[test]
    fn test_suggested_action_for_auth() {
        let err = ProviderError::Authentication("token expired".to_string());
        assert!(err.suggested_action().contains("gh auth login"));
    }

    #[test]
    fn test_suggested_action_for_rate_limit() {
        let err = ProviderError::RateLimited {
            reset_time: "2024-01-01T00:00:00Z".to_string(),
        };
        assert!(err.suggested_action().contains("rate limit"));
    }

    #[test]
    fn test_from_status_creates_correct_error_type() {
        let err = ProviderError::from_status(401, "Unauthorized");
        assert!(matches!(err, ProviderError::Authentication(_)));

        let err = ProviderError::from_status(403, "Forbidden");
        assert!(matches!(err, ProviderError::PermissionDenied(_)));

        let err = ProviderError::from_status(404, "Not Found");
        assert!(matches!(err, ProviderError::NotFound(_)));

        let err = ProviderError::from_status(500, "Server Error");
        assert!(matches!(err, ProviderError::Api { status: 500, .. }));
    }

    #[test]
    fn test_error_display() {
        let err = ProviderError::Api {
            status: 500,
            message: "Internal Server Error".to_string(),
        };
        let display = format!("{}", err);
        assert!(display.contains("500"));
        assert!(display.contains("Internal Server Error"));
    }
}
