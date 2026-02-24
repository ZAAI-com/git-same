//! Git operation error types.
//!
//! These errors represent failures that occur when executing git commands
//! via the shell (clone, fetch, pull, etc.).

use thiserror::Error;

/// Errors that occur during git operations.
#[derive(Error, Debug)]
pub enum GitError {
    /// Git executable not found in PATH.
    #[error("Git not found. Please install git and ensure it's in your PATH")]
    GitNotFound,

    /// Clone operation failed.
    #[error("Clone failed for {repo}: {message}")]
    CloneFailed {
        /// Repository URL or name
        repo: String,
        /// Error message from git
        message: String,
    },

    /// Fetch operation failed.
    #[error("Fetch failed for {repo}: {message}")]
    FetchFailed {
        /// Repository path or name
        repo: String,
        /// Error message from git
        message: String,
    },

    /// Pull operation failed.
    #[error("Pull failed for {repo}: {message}")]
    PullFailed {
        /// Repository path or name
        repo: String,
        /// Error message from git
        message: String,
    },

    /// Repository has uncommitted changes that would be overwritten.
    #[error("Repository has uncommitted changes: {path}")]
    UncommittedRepository {
        /// Path to the repository
        path: String,
    },

    /// Path is not a git repository.
    #[error("Not a git repository: {path}")]
    NotARepository {
        /// Path that was expected to be a repository
        path: String,
    },

    /// Permission denied during git operation.
    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    /// SSH key not configured for the host.
    #[error("SSH key not configured for {host}. Run 'ssh -T git@{host}' to test")]
    SshKeyMissing {
        /// The git host (e.g., github.com)
        host: String,
    },

    /// SSH authentication failed.
    #[error("SSH authentication failed for {host}: {message}")]
    SshAuthFailed {
        /// The git host
        host: String,
        /// Error message
        message: String,
    },

    /// Generic command execution failure.
    #[error("Git command failed: {0}")]
    CommandFailed(String),

    /// Timeout during git operation.
    #[error("Git operation timed out after {seconds} seconds")]
    Timeout {
        /// Number of seconds before timeout
        seconds: u64,
    },
}

impl GitError {
    /// Creates a clone failed error.
    pub fn clone_failed(repo: impl Into<String>, message: impl Into<String>) -> Self {
        GitError::CloneFailed {
            repo: repo.into(),
            message: message.into(),
        }
    }

    /// Creates a fetch failed error.
    pub fn fetch_failed(repo: impl AsRef<std::path::Path>, message: impl Into<String>) -> Self {
        GitError::FetchFailed {
            repo: repo.as_ref().to_string_lossy().to_string(),
            message: message.into(),
        }
    }

    /// Creates a pull failed error.
    pub fn pull_failed(repo: impl AsRef<std::path::Path>, message: impl Into<String>) -> Self {
        GitError::PullFailed {
            repo: repo.as_ref().to_string_lossy().to_string(),
            message: message.into(),
        }
    }

    /// Creates a command failed error.
    pub fn command_failed(command: impl Into<String>, message: impl Into<String>) -> Self {
        GitError::CommandFailed(format!("{}: {}", command.into(), message.into()))
    }

    /// Returns `true` if this error indicates the repository can be skipped
    /// safely without affecting other operations.
    pub fn is_skippable(&self) -> bool {
        matches!(
            self,
            GitError::UncommittedRepository { .. }
                | GitError::PermissionDenied(_)
                | GitError::SshKeyMissing { .. }
                | GitError::SshAuthFailed { .. }
        )
    }

    /// Returns `true` if this error might be resolved by retrying.
    pub fn is_retryable(&self) -> bool {
        matches!(self, GitError::Timeout { .. } | GitError::CommandFailed(_))
    }

    /// Returns a user-friendly suggestion for how to resolve this error.
    pub fn suggested_action(&self) -> &'static str {
        match self {
            GitError::GitNotFound => "Install git from https://git-scm.com/downloads",
            GitError::CloneFailed { .. } => "Check the repository URL and your network connection",
            GitError::FetchFailed { .. } | GitError::PullFailed { .. } => {
                "Check your network connection and repository access"
            }
            GitError::UncommittedRepository { .. } => "Commit or stash your changes before syncing",
            GitError::NotARepository { .. } => {
                "The directory exists but is not a git repository. Remove it to clone fresh"
            }
            GitError::PermissionDenied(_) => "Check file permissions and your authentication",
            GitError::SshKeyMissing { .. } => {
                "Add your SSH key to the git hosting service, or use HTTPS authentication"
            }
            GitError::SshAuthFailed { .. } => {
                "Check your SSH key configuration with 'ssh -T git@github.com'"
            }
            GitError::CommandFailed(_) => "Check the error message and try again",
            GitError::Timeout { .. } => {
                "The operation took too long. Try with a smaller repository or better connection"
            }
        }
    }

    /// Extracts the repository identifier from the error, if available.
    pub fn repo_identifier(&self) -> Option<&str> {
        match self {
            GitError::CloneFailed { repo, .. }
            | GitError::FetchFailed { repo, .. }
            | GitError::PullFailed { repo, .. } => Some(repo),
            GitError::UncommittedRepository { path } | GitError::NotARepository { path } => {
                Some(path)
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
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
}
