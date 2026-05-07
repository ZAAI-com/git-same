//! Git hosting provider implementations.
//!
//! This module contains the [`Provider`] trait and implementations for
//! various Git hosting services:
//!
//! - **GitHub** - github.com (active)
//! - **GitHub Enterprise** - coming soon
//! - **GitLab** - coming soon
//! - **Codeberg** - coming soon
//! - **Bitbucket** - coming soon
//!
//! # Example
//!
//! ```no_run
//! use git_same_core::provider::{create_provider, DiscoveryOptions, NoProgress};
//! use git_same_core::config::WorkspaceProvider;
//!
//! # async fn example() -> Result<(), git_same_core::errors::AppError> {
//! let provider = WorkspaceProvider::default();
//! let p = create_provider(&provider, "ghp_token123")?;
//!
//! let options = DiscoveryOptions::new();
//! let progress = NoProgress;
//! let repos = p.discover_repos(&options, &progress).await?;
//! # Ok(())
//! # }
//! ```

pub mod github;
pub mod traits;

#[cfg(test)]
pub mod mock;

pub use traits::{
    Credentials, DiscoveryOptions, DiscoveryProgress, NoProgress, Provider, RateLimitInfo,
};

use crate::config::WorkspaceProvider;
use crate::errors::{AppError, ProviderError};
use crate::types::ProviderKind;

/// Creates a provider instance based on workspace provider configuration.
pub fn create_provider(
    ws_provider: &WorkspaceProvider,
    token: &str,
) -> Result<Box<dyn Provider>, AppError> {
    let api_url = ws_provider.effective_api_url();

    match ws_provider.kind {
        ProviderKind::GitHub => {
            let credentials = Credentials::new(token, api_url);
            let provider =
                github::GitHubProvider::new(credentials, ws_provider.display_name().to_string())
                    .map_err(AppError::Provider)?;
            Ok(Box::new(provider))
        }
        other => Err(AppError::Provider(ProviderError::NotImplemented(format!(
            "{} support coming soon",
            other.display_name()
        )))),
    }
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
