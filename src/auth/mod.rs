//! Authentication management for gisa.
//!
//! This module handles authentication with Git hosting providers
//! using the GitHub CLI (`gh auth token`).
//!
//! # Example
//!
//! ```no_run
//! use git_same::auth::{get_auth_for_provider, AuthResult};
//! use git_same::config::ProviderEntry;
//!
//! let provider = ProviderEntry::github();
//! let auth = get_auth_for_provider(&provider).expect("Failed to authenticate");
//! println!("Authenticated as {:?} via {}", auth.username, auth.method);
//! ```

pub mod gh_cli;
pub mod ssh;

use crate::config::ProviderEntry;
use crate::errors::AppError;
use tracing::{debug, warn};

/// Authentication result containing the token and metadata.
#[derive(Debug, Clone)]
pub struct AuthResult {
    /// The authentication token
    pub token: String,
    /// Method used to obtain the token
    pub method: ResolvedAuthMethod,
    /// The authenticated username (if available)
    pub username: Option<String>,
}

/// The actual method used for authentication.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolvedAuthMethod {
    /// Used GitHub CLI
    GhCli,
}

impl std::fmt::Display for ResolvedAuthMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResolvedAuthMethod::GhCli => write!(f, "GitHub CLI"),
        }
    }
}

/// Get authentication using the GitHub CLI.
///
/// Requires `gh` to be installed and authenticated.
pub fn get_auth() -> Result<AuthResult, AppError> {
    debug!("Resolving authentication via gh CLI");

    // Try gh CLI
    let gh_installed = gh_cli::is_installed();
    let gh_authenticated = gh_installed && gh_cli::is_authenticated();
    debug!(gh_installed, gh_authenticated, "Checking GitHub CLI status");

    if gh_installed && gh_authenticated {
        match gh_cli::get_token() {
            Ok(token) => {
                let username = gh_cli::get_username().ok();
                debug!(
                    username = username.as_deref().unwrap_or("<unknown>"),
                    "Authenticated via GitHub CLI"
                );
                return Ok(AuthResult {
                    token,
                    method: ResolvedAuthMethod::GhCli,
                    username,
                });
            }
            Err(e) => {
                warn!(error = %e, "gh CLI token retrieval failed");
            }
        }
    }

    // No authentication found - provide helpful error message
    let ssh_note = if ssh::has_ssh_keys() {
        "\n\nNote: SSH keys detected. While SSH keys work for git clone/push,\n\
         you still need a provider API token for repository discovery.\n\
         The SSH keys will be used automatically for cloning."
    } else {
        ""
    };

    Err(AppError::auth(format!(
        "No authentication found.\n\n\
         Please authenticate using the GitHub CLI:\n\n\
         For GitHub.com:       gh auth login\n\
         For GitHub Enterprise: gh auth login --hostname <your-host>\n\
         {}\n\
         Install from: https://cli.github.com/",
        ssh_note
    )))
}

/// Get authentication for a specific provider configuration.
pub fn get_auth_for_provider(provider: &ProviderEntry) -> Result<AuthResult, AppError> {
    debug!(
        api_url = provider.api_url.as_deref().unwrap_or("default"),
        "Resolving authentication for provider"
    );

    // For GitHub Enterprise, try to get token for specific host
    if let Some(api_url) = &provider.api_url {
        if let Some(host) = extract_host(api_url) {
            if host != "api.github.com" {
                debug!(host, "Attempting GitHub Enterprise authentication");
                if let Ok(token) = gh_cli::get_token_for_host(&host) {
                    debug!(host, "Authenticated via gh CLI for enterprise host");
                    return Ok(AuthResult {
                        token,
                        method: ResolvedAuthMethod::GhCli,
                        username: None,
                    });
                }
            }
        }
    }

    // Default gh auth
    if !gh_cli::is_installed() {
        debug!("gh CLI not installed");
        return Err(AppError::auth(
            "GitHub CLI is not installed. Install from https://cli.github.com/",
        ));
    }
    if !gh_cli::is_authenticated() {
        debug!("gh CLI not authenticated");
        return Err(AppError::auth(
            "GitHub CLI is not authenticated. Run: gh auth login",
        ));
    }

    let token = gh_cli::get_token()?;
    let username = gh_cli::get_username().ok();
    debug!(
        username = username.as_deref().unwrap_or("<unknown>"),
        "Authenticated via gh CLI"
    );

    Ok(AuthResult {
        token,
        method: ResolvedAuthMethod::GhCli,
        username,
    })
}

/// Extract hostname from an API URL.
fn extract_host(url: &str) -> Option<String> {
    let without_scheme = url.split_once("://").map(|(_, rest)| rest).unwrap_or(url);
    let host = without_scheme.split('/').next()?;
    if host.is_empty() {
        return None;
    }
    Some(host.to_string())
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
