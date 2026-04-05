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
/// and SSH GitHub access.
pub async fn check_requirements() -> Vec<CheckResult> {
    match tokio::task::spawn_blocking(check_requirements_sync).await {
        Ok(results) => results,
        Err(e) => vec![CheckResult {
            name: "System checks".to_string(),
            passed: false,
            message: format!("failed to run checks: {}", e),
            suggestion: Some("Try running checks again".to_string()),
            critical: false,
        }],
    }
}

/// Run all requirement checks synchronously.
pub fn check_requirements_sync() -> Vec<CheckResult> {
    vec![
        check_git_installed(),
        check_gh_installed(),
        check_gh_authenticated(),
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
#[path = "checks_tests.rs"]
mod tests;
