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
fn test_429_is_retryable() {
    let err = ProviderError::Api {
        status: 429,
        message: "Too Many Requests".to_string(),
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
    assert!(err.suggested_action().contains("Re-authenticate"));
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
