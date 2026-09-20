use super::*;
use crate::errors::AppError;

#[test]
fn exit_codes_distinguish_busy_and_unsupported() {
    assert_eq!(MonitorAgentError::Busy.exit_code(), 9);
    assert_eq!(MonitorAgentError::Unsupported.exit_code(), 10);
    assert_eq!(MonitorAgentError::MissingSource("x".into()).exit_code(), 8);
}

#[test]
fn busy_and_timeout_are_retryable() {
    assert!(MonitorAgentError::Busy.is_retryable());
    assert!(MonitorAgentError::CommandTimeout {
        command: "launchctl".into(),
        timeout: Duration::from_secs(10),
    }
    .is_retryable());
    assert!(!MonitorAgentError::Unsupported.is_retryable());
}

#[test]
fn transaction_error_reports_original_and_rollback() {
    let message = MonitorAgentError::Transaction {
        original: "bootstrap failed".into(),
        rollback: Some("rename failed".into()),
    }
    .to_string();
    assert!(message.contains("bootstrap failed"));
    assert!(message.contains("rollback also failed: rename failed"));

    let message = MonitorAgentError::Transaction {
        original: "copy failed".into(),
        rollback: None,
    }
    .to_string();
    assert!(!message.contains("rollback"));
}

#[test]
fn converts_into_app_error_with_matching_exit_code() {
    let error: AppError = MonitorAgentError::Busy.into();
    assert_eq!(error.exit_code(), 9);
    assert!(error.is_retryable());
    assert_eq!(error.suggested_action(), "Wait a moment and try again");
}
