//! Authentication management for gisa.
//!
//! This module handles authentication with Git hosting providers,
//! supporting multiple authentication methods:
//!
//! 1. **GitHub CLI** (`gh auth token`) - Recommended, secure
//! 2. **Environment variables** - CI-friendly
//! 3. **Config file tokens** - Not recommended, last resort
//!
//! # Example
//!
//! ```no_run
//! use git_same::auth::{get_auth, AuthResult};
//!
//! let auth = get_auth(None).expect("Failed to authenticate");
//! println!("Authenticated as {:?} via {}", auth.username, auth.method);
//! ```

pub mod env_token;
pub mod gh_cli;
pub mod ssh;

use crate::config::{AuthMethod, ProviderEntry};
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
    /// Used environment variable (with name)
    EnvVar(String),
    /// Used token from config file
    ConfigToken,
}

impl std::fmt::Display for ResolvedAuthMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResolvedAuthMethod::GhCli => write!(f, "GitHub CLI"),
            ResolvedAuthMethod::EnvVar(name) => write!(f, "env:{}", name),
            ResolvedAuthMethod::ConfigToken => write!(f, "config token"),
        }
    }
}

/// Get authentication using the default priority order.
///
/// Priority: gh CLI → environment variables → config token
///
/// # Arguments
/// * `config_token` - Optional token from config file (last resort)
pub fn get_auth(config_token: Option<&str>) -> Result<AuthResult, AppError> {
    debug!("Resolving authentication (priority: gh CLI → env vars → config token)");

    // Try gh CLI first
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
                // gh CLI is installed and authenticated but token retrieval failed
                // This can happen with permission issues or corrupted auth state
                warn!(
                    error = %e,
                    "gh CLI token retrieval failed, trying alternative methods"
                );
                eprintln!(
                    "Note: gh CLI token retrieval failed ({}), trying alternative methods",
                    e
                );
            }
        }
    }

    // Try environment variables
    debug!("Checking environment variables for token");
    if let Ok((token, var_name)) = env_token::get_token_from_defaults() {
        debug!(var_name, "Authenticated via environment variable");
        return Ok(AuthResult {
            token,
            method: ResolvedAuthMethod::EnvVar(var_name.to_string()),
            username: None, // Will be fetched via API later
        });
    }

    // Try config token
    if let Some(token) = config_token {
        if !token.is_empty() {
            debug!("Authenticated via config file token");
            return Ok(AuthResult {
                token: token.to_string(),
                method: ResolvedAuthMethod::ConfigToken,
                username: None,
            });
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
        "No authentication found for your Git provider.\n\n\
         Please authenticate using one of these methods:\n\n\
         1. Provider CLI (recommended):\n   \
            For GitHub.com: gh auth login\n   \
            For GitHub Enterprise: gh auth login --hostname <your-host>\n\n\
         2. Environment variable:\n   \
            export <PROVIDER_TOKEN>=<token>\n\
            (For GitHub, common names are GITHUB_TOKEN or GH_TOKEN)\n\
         {}\n\
         For more info (GitHub CLI): https://cli.github.com/manual/gh_auth_login",
        ssh_note
    )))
}

/// Get authentication for a specific provider configuration.
pub fn get_auth_for_provider(provider: &ProviderEntry) -> Result<AuthResult, AppError> {
    debug!(
        auth_method = ?provider.auth,
        api_url = provider.api_url.as_deref().unwrap_or("default"),
        "Resolving authentication for provider"
    );

    match provider.auth {
        AuthMethod::GhCli => {
            // For GitHub Enterprise, we might need to specify the host
            if let Some(api_url) = &provider.api_url {
                // Extract host from API URL
                if let Some(host) = extract_host(api_url) {
                    if host != "api.github.com" {
                        debug!(host, "Attempting GitHub Enterprise authentication");
                        // Try to get token for specific host
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

        AuthMethod::Env => {
            let var_name = provider.token_env.as_deref().unwrap_or("GITHUB_TOKEN");
            debug!(var_name, "Attempting environment variable authentication");

            let token = env_token::get_token(var_name)?;
            debug!(var_name, "Authenticated via environment variable");

            Ok(AuthResult {
                token,
                method: ResolvedAuthMethod::EnvVar(var_name.to_string()),
                username: None,
            })
        }

        AuthMethod::Token => {
            debug!("Using config file token authentication");
            let token = provider
                .token
                .clone()
                .ok_or_else(|| AppError::auth("Token auth configured but no token provided"))?;
            debug!("Authenticated via config token");

            Ok(AuthResult {
                token,
                method: ResolvedAuthMethod::ConfigToken,
                username: None,
            })
        }
    }
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
