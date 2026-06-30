//! Configuration management for git-same.
//!
//! This module handles loading, parsing, and validating configuration
//! from `git-same.config.toml` files.
//!
//! # Example Configuration
//!
//! ```toml
//! structure = "{org}/{repo}"
//! concurrency = 4
//! ```

mod parser;
mod provider_config;
pub mod workspace;
pub mod workspace_manager;
pub mod workspace_policy;
pub mod workspace_store;

pub use parser::{Config, ConfigCloneOptions, FilterOptions, SyncMode};
pub use workspace::{WorkspaceConfig, WorkspaceProvider};
pub use workspace_manager::WorkspaceManager;
pub use workspace_policy::WorkspacePolicy;
pub use workspace_store::WorkspaceStore;
