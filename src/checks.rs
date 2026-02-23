//! System requirements checking.
//!
//! Provides reusable requirement checks for both the CLI `init` command
//! and the TUI init screen.

use crate::auth::{gh_cli, ssh};
use std::process::Command;

/// Result of a single requirement check.
#[derive(Debug, Clone)]
pub struct CheckResult {
    /// Human-readable name of the check (e.g., "Git CLI").
    pub name: String,
    /// Whether the check passed.
    pub passed: bool,
    /// Detail message (e.g., "git 2.43.0" or "not found").
    pub message: String,
    /// Suggested action to fix a failure.
    pub suggestion: Option<String>,
    /// Whether this is a critical requirement (false = warning only).
    pub critical: bool,
}

/// Run all requirement checks.
///
/// Returns a list of check results for: git, gh CLI, gh authentication,
/// SSH keys, and SSH GitHub access.
pub async fn check_requirements() -> Vec<CheckResult> {
    vec![
        check_git_installed(),
        check_gh_installed(),
        check_gh_authenticated(),
        check_ssh_keys(),
        check_ssh_github_access(),
    ]
}

/// Check if git is installed and get its version.
fn check_git_installed() -> CheckResult {
    match Command::new("git").arg("--version").output() {
        Ok(output) if output.status.success() => {
            let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
            CheckResult {
                name: "Git".to_string(),
                passed: true,
                message: version,
                suggestion: None,
                critical: true,
            }
        }
        _ => CheckResult {
            name: "Git".to_string(),
            passed: false,
            message: "not found".to_string(),
            suggestion: Some("Install git: https://git-scm.com/downloads".to_string()),
            critical: true,
        },
    }
}

/// Check if the GitHub CLI is installed.
fn check_gh_installed() -> CheckResult {
    if gh_cli::is_installed() {
        let version = Command::new("gh")
            .arg("--version")
            .output()
            .ok()
            .map(|o| {
                String::from_utf8_lossy(&o.stdout)
                    .lines()
                    .next()
                    .unwrap_or("")
                    .trim()
                    .to_string()
            })
            .unwrap_or_else(|| "installed".to_string());
        CheckResult {
            name: "GitHub CLI".to_string(),
            passed: true,
            message: version,
            suggestion: None,
            critical: true,
        }
    } else {
        CheckResult {
            name: "GitHub CLI".to_string(),
            passed: false,
            message: "not found".to_string(),
            suggestion: Some("Install from https://cli.github.com/".to_string()),
            critical: true,
        }
    }
}

/// Check if the user is authenticated with the GitHub CLI.
fn check_gh_authenticated() -> CheckResult {
    if !gh_cli::is_installed() {
        return CheckResult {
            name: "GitHub Auth".to_string(),
            passed: false,
            message: "gh CLI not installed".to_string(),
            suggestion: Some("Install gh CLI first, then run: gh auth login".to_string()),
            critical: true,
        };
    }

    if gh_cli::is_authenticated() {
        let username = gh_cli::get_username().unwrap_or_else(|_| "authenticated".to_string());
        CheckResult {
            name: "GitHub Auth".to_string(),
            passed: true,
            message: format!("logged in as {}", username),
            suggestion: None,
            critical: true,
        }
    } else {
        CheckResult {
            name: "GitHub Auth".to_string(),
            passed: false,
            message: "not authenticated".to_string(),
            suggestion: Some("Run: gh auth login".to_string()),
            critical: true,
        }
    }
}

/// Check if SSH keys are present.
fn check_ssh_keys() -> CheckResult {
    if ssh::has_ssh_keys() {
        let keys = ssh::get_ssh_key_files();
        let key_names: Vec<String> = keys
            .iter()
            .filter_map(|p| p.file_name().map(|f| f.to_string_lossy().to_string()))
            .collect();
        CheckResult {
            name: "SSH Keys".to_string(),
            passed: true,
            message: key_names.join(", "),
            suggestion: None,
            critical: false,
        }
    } else {
        CheckResult {
            name: "SSH Keys".to_string(),
            passed: false,
            message: "no SSH keys found in ~/.ssh".to_string(),
            suggestion: Some(
                "Generate a key: ssh-keygen -t ed25519 -C \"your_email@example.com\"".to_string(),
            ),
            critical: false,
        }
    }
}

/// Check if SSH access to GitHub works.
fn check_ssh_github_access() -> CheckResult {
    if ssh::has_github_ssh_access() {
        CheckResult {
            name: "SSH GitHub".to_string(),
            passed: true,
            message: "authenticated".to_string(),
            suggestion: None,
            critical: false,
        }
    } else {
        CheckResult {
            name: "SSH GitHub".to_string(),
            passed: false,
            message: "cannot reach github.com via SSH".to_string(),
            suggestion: Some(
                "Add your SSH key to GitHub: https://github.com/settings/keys".to_string(),
            ),
            critical: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_git_installed_runs() {
        let result = check_git_installed();
        // Just verify it runs without panic; actual result depends on environment
        assert_eq!(result.name, "Git");
        assert!(result.critical);
    }

    #[test]
    fn test_check_gh_installed_runs() {
        let result = check_gh_installed();
        assert_eq!(result.name, "GitHub CLI");
        assert!(result.critical);
    }

    #[test]
    fn test_check_ssh_keys_runs() {
        let result = check_ssh_keys();
        assert_eq!(result.name, "SSH Keys");
        assert!(!result.critical);
    }

    #[test]
    fn test_check_result_fields() {
        let result = CheckResult {
            name: "Test".to_string(),
            passed: true,
            message: "ok".to_string(),
            suggestion: None,
            critical: false,
        };
        assert!(result.passed);
        assert!(result.suggestion.is_none());
    }

    #[tokio::test]
    async fn test_check_requirements_returns_all_checks() {
        let results = check_requirements().await;
        assert_eq!(results.len(), 5);
        assert_eq!(results[0].name, "Git");
        assert_eq!(results[1].name, "GitHub CLI");
        assert_eq!(results[2].name, "GitHub Auth");
        assert_eq!(results[3].name, "SSH Keys");
        assert_eq!(results[4].name, "SSH GitHub");
    }
}
