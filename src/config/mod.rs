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
pub mod workspace;
pub mod workspace_manager;
pub mod workspace_policy;
pub mod workspace_store;

pub use parser::{Config, ConfigCloneOptions, FilterOptions, SyncMode};
pub use provider_config::{AuthMethod, ProviderEntry};
pub use workspace::{WorkspaceConfig, WorkspaceProvider};
pub use workspace_manager::WorkspaceManager;
pub use workspace_policy::WorkspacePolicy;
pub use workspace_store::WorkspaceStore;
