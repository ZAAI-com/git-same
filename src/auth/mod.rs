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
    // Try gh CLI first
    if gh_cli::is_installed() && gh_cli::is_authenticated() {
        match gh_cli::get_token() {
            Ok(token) => {
                let username = gh_cli::get_username().ok();
                return Ok(AuthResult {
                    token,
                    method: ResolvedAuthMethod::GhCli,
                    username,
                });
            }
            Err(_) => {
                // Fall through to next method
            }
        }
    }

    // Try environment variables
    if let Ok((token, var_name)) = env_token::get_token_from_defaults() {
        return Ok(AuthResult {
            token,
            method: ResolvedAuthMethod::EnvVar(var_name.to_string()),
            username: None, // Will be fetched via API later
        });
    }

    // Try config token
    if let Some(token) = config_token {
        if !token.is_empty() {
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
         you still need a GitHub token for API access (discovering repos).\n\
         The SSH keys will be used automatically for cloning."
    } else {
        ""
    };

    Err(AppError::auth(format!(
        "No GitHub authentication found.\n\n\
         Please authenticate using one of these methods:\n\n\
         1. GitHub CLI (recommended):\n   \
            gh auth login\n\n\
         2. Environment variable:\n   \
            export GITHUB_TOKEN=ghp_xxxx\n\
         {}\n\
         For more info: https://cli.github.com/manual/gh_auth_login",
        ssh_note
    )))
}

/// Get authentication for a specific provider configuration.
pub fn get_auth_for_provider(provider: &ProviderEntry) -> Result<AuthResult, AppError> {
    match provider.auth {
        AuthMethod::GhCli => {
            // For GitHub Enterprise, we might need to specify the host
            if let Some(api_url) = &provider.api_url {
                // Extract host from API URL
                if let Some(host) = extract_host(api_url) {
                    if host != "api.github.com" {
                        // Try to get token for specific host
                        if let Ok(token) = gh_cli::get_token_for_host(&host) {
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
                return Err(AppError::auth(
                    "GitHub CLI is not installed. Install from https://cli.github.com/",
                ));
            }
            if !gh_cli::is_authenticated() {
                return Err(AppError::auth(
                    "GitHub CLI is not authenticated. Run: gh auth login",
                ));
            }

            let token = gh_cli::get_token()?;
            let username = gh_cli::get_username().ok();

            Ok(AuthResult {
                token,
                method: ResolvedAuthMethod::GhCli,
                username,
            })
        }

        AuthMethod::Env => {
            let var_name = provider.token_env.as_deref().unwrap_or("GITHUB_TOKEN");

            let token = env_token::get_token(var_name)?;

            Ok(AuthResult {
                token,
                method: ResolvedAuthMethod::EnvVar(var_name.to_string()),
                username: None,
            })
        }

        AuthMethod::Token => {
            let token = provider
                .token
                .clone()
                .ok_or_else(|| AppError::auth("Token auth configured but no token provided"))?;

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
    // Simple extraction - could use url crate for more robust parsing
    let url = url
        .trim_start_matches("https://")
        .trim_start_matches("http://");
    let host = url.split('/').next()?;
    Some(host.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolved_auth_method_display() {
        assert_eq!(format!("{}", ResolvedAuthMethod::GhCli), "GitHub CLI");
        assert_eq!(
            format!("{}", ResolvedAuthMethod::EnvVar("MY_TOKEN".to_string())),
            "env:MY_TOKEN"
        );
        assert_eq!(
            format!("{}", ResolvedAuthMethod::ConfigToken),
            "config token"
        );
    }

    #[test]
    fn test_extract_host() {
        assert_eq!(
            extract_host("https://api.github.com"),
            Some("api.github.com".to_string())
        );
        assert_eq!(
            extract_host("https://github.company.com/api/v3"),
            Some("github.company.com".to_string())
        );
        assert_eq!(
            extract_host("http://localhost:8080/api"),
            Some("localhost:8080".to_string())
        );
    }

    #[test]
    fn test_get_auth_with_config_token() {
        // Clear env vars temporarily for this test
        let saved_github_token = std::env::var("GITHUB_TOKEN").ok();
        let saved_gh_token = std::env::var("GH_TOKEN").ok();
        let saved_gisa_token = std::env::var("GISA_TOKEN").ok();

        std::env::remove_var("GITHUB_TOKEN");
        std::env::remove_var("GH_TOKEN");
        std::env::remove_var("GISA_TOKEN");

        // If gh is not installed/authenticated, this should use config token
        let result = get_auth(Some("test_token_value"));

        // Restore env vars
        if let Some(v) = saved_github_token {
            std::env::set_var("GITHUB_TOKEN", v);
        }
        if let Some(v) = saved_gh_token {
            std::env::set_var("GH_TOKEN", v);
        }
        if let Some(v) = saved_gisa_token {
            std::env::set_var("GISA_TOKEN", v);
        }

        // The result depends on whether gh is installed
        // If no gh, it should use config token or return error
        if let Ok(auth) = result {
            // Could be GhCli if gh is available, or ConfigToken
            assert!(!auth.token.is_empty());
        }
    }

    #[test]
    fn test_get_auth_for_provider_env() {
        let unique_var = "GISA_TEST_PROVIDER_TOKEN";
        std::env::set_var(unique_var, "test_provider_token");

        let provider = ProviderEntry {
            auth: AuthMethod::Env,
            token_env: Some(unique_var.to_string()),
            ..ProviderEntry::default()
        };

        let result = get_auth_for_provider(&provider);
        assert!(result.is_ok());

        let auth = result.unwrap();
        assert_eq!(auth.token, "test_provider_token");
        assert_eq!(
            auth.method,
            ResolvedAuthMethod::EnvVar(unique_var.to_string())
        );

        std::env::remove_var(unique_var);
    }

    #[test]
    fn test_get_auth_for_provider_config_token() {
        let provider = ProviderEntry {
            auth: AuthMethod::Token,
            token: Some("my_config_token".to_string()),
            ..ProviderEntry::default()
        };

        let result = get_auth_for_provider(&provider);
        assert!(result.is_ok());

        let auth = result.unwrap();
        assert_eq!(auth.token, "my_config_token");
        assert_eq!(auth.method, ResolvedAuthMethod::ConfigToken);
    }

    #[test]
    fn test_get_auth_for_provider_missing_token() {
        let provider = ProviderEntry {
            auth: AuthMethod::Token,
            token: None,
            ..ProviderEntry::default()
        };

        let result = get_auth_for_provider(&provider);
        assert!(result.is_err());
    }

    #[test]
    fn test_get_auth_for_provider_missing_env() {
        let provider = ProviderEntry {
            auth: AuthMethod::Env,
            token_env: Some("NONEXISTENT_VAR_XXXXX".to_string()),
            ..ProviderEntry::default()
        };

        let result = get_auth_for_provider(&provider);
        assert!(result.is_err());
    }
}
