use super::*;

#[test]
fn test_uncommitted_repository_is_skippable() {
    let err = GitError::UncommittedRepository {
        path: "/home/user/repo".to_string(),
    };
    assert!(err.is_skippable());
}

#[test]
fn test_ssh_errors_are_skippable() {
    let err = GitError::SshKeyMissing {
        host: "github.com".to_string(),
    };
    assert!(err.is_skippable());

    let err = GitError::SshAuthFailed {
        host: "github.com".to_string(),
        message: "Permission denied".to_string(),
    };
    assert!(err.is_skippable());
}

#[test]
fn test_clone_failed_is_not_skippable() {
    let err = GitError::CloneFailed {
        repo: "org/repo".to_string(),
        message: "Network error".to_string(),
    };
    assert!(!err.is_skippable());
}

#[test]
fn test_timeout_is_retryable() {
    let err = GitError::Timeout { seconds: 120 };
    assert!(err.is_retryable());
}

#[test]
fn test_git_not_found_is_not_retryable() {
    let err = GitError::GitNotFound;
    assert!(!err.is_retryable());
}

#[test]
fn test_command_failed_is_not_retryable() {
    let err = GitError::CommandFailed("some failure".to_string());
    assert!(!err.is_retryable());
}

#[test]
fn test_repo_identifier_extraction() {
    let err = GitError::CloneFailed {
        repo: "my-org/my-repo".to_string(),
        message: "error".to_string(),
    };
    assert_eq!(err.repo_identifier(), Some("my-org/my-repo"));

    let err = GitError::UncommittedRepository {
        path: "/path/to/repo".to_string(),
    };
    assert_eq!(err.repo_identifier(), Some("/path/to/repo"));

    let err = GitError::GitNotFound;
    assert_eq!(err.repo_identifier(), None);
}

#[test]
fn test_error_display() {
    let err = GitError::CloneFailed {
        repo: "org/repo".to_string(),
        message: "fatal: repository not found".to_string(),
    };
    let display = format!("{}", err);
    assert!(display.contains("org/repo"));
    assert!(display.contains("repository not found"));
}

#[test]
fn test_suggested_actions_are_helpful() {
    let err = GitError::SshKeyMissing {
        host: "github.com".to_string(),
    };
    let suggestion = err.suggested_action();
    assert!(suggestion.contains("SSH") || suggestion.contains("HTTPS"));
}
