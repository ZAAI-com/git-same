//! Configuration management for gisa.
//!
//! This module handles loading, parsing, and validating configuration
//! from `gisa.config.toml` files.
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

pub use parser::{CloneOptions, Config, FilterOptions, SyncMode};
pub use provider_config::{AuthMethod, ProviderEntry};
