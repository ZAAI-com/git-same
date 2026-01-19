//! Git hosting provider implementations.
//!
//! This module contains the [`Provider`] trait and implementations for
//! various Git hosting services:
//!
//! - **GitHub** - github.com and GitHub Enterprise
//! - **GitLab** - gitlab.com and self-hosted (future)
//! - **Bitbucket** - bitbucket.org (future)
//!
//! # Example
//!
//! ```no_run
//! use gisa::provider::{create_provider, DiscoveryOptions, NoProgress};
//! use gisa::config::ProviderEntry;
//!
//! # async fn example() -> Result<(), gisa::errors::AppError> {
//! let entry = ProviderEntry::github();
//! let provider = create_provider(&entry, "ghp_token123")?;
//!
//! let options = DiscoveryOptions::new();
//! let progress = NoProgress;
//! let repos = provider.discover_repos(&options, &progress).await?;
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

use crate::config::ProviderEntry;
use crate::errors::{AppError, ProviderError};
use crate::types::ProviderKind;

/// Creates a provider instance based on configuration.
pub fn create_provider(
    entry: &ProviderEntry,
    token: &str,
) -> Result<Box<dyn Provider>, AppError> {
    let api_url = entry.effective_api_url();

    match entry.kind {
        ProviderKind::GitHub | ProviderKind::GitHubEnterprise => {
            let credentials = Credentials::new(token, api_url);
            let provider = github::GitHubProvider::new(credentials, entry.display_name())
                .map_err(|e| AppError::Provider(e))?;
            Ok(Box::new(provider))
        }
        ProviderKind::GitLab => Err(AppError::Provider(ProviderError::NotImplemented(
            "GitLab support coming soon".to_string(),
        ))),
        ProviderKind::Bitbucket => Err(AppError::Provider(ProviderError::NotImplemented(
            "Bitbucket support coming soon".to_string(),
        ))),
    }
}
