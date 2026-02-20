//! Configuration management for git-same.
//!
//! This module handles loading, parsing, and validating configuration
//! from `git-same.config.toml` files.
//!
//! # Example Configuration
//!
//! ```toml
//! base_path = "~/github"
//! concurrency = 4
//!
//! [[providers]]
//! kind = "github"
//! auth = "gh-cli"
//! ```

mod parser;
mod provider_config;

pub use parser::{Config, ConfigCloneOptions, FilterOptions, SyncMode};
pub use provider_config::{AuthMethod, ProviderEntry};
