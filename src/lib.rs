//! # Git-Same - Mirror GitHub org/repo structure locally
//!
//! Git-Same is a CLI tool that discovers all GitHub organizations
//! and repositories you have access to, then clones them to your local filesystem
//! maintaining the org/repo directory structure.
//!
//! ## Features
//!
//! - **Multi-Provider Support**: Works with GitHub, GitHub Enterprise, GitLab, and Bitbucket
//! - **Parallel Operations**: Clones and syncs repositories concurrently
//! - **Smart Filtering**: Filter by archived status, forks, organizations
//! - **Incremental Sync**: Only fetches/pulls what has changed
//! - **Progress Reporting**: Beautiful progress bars and status updates
//!
//! ## Available Commands
//!
//! The tool can be invoked using any of these names (all installed by default):
//! - `git-same` - Main command
//! - `gitsame` - No hyphen variant
//! - `gitsa` - Short form
//! - `gisa` - Shortest variant
//! - `git same` - Git subcommand (requires git-same in PATH)
//!
//! ## Example
//!
//! ```bash
//! # Initialize configuration
//! git-same init
//!
//! # Set up a workspace
//! git-same setup
//!
//! # Sync repositories (clone new + fetch existing)
//! git-same sync --dry-run
//! git-same sync
//!
//! # Show status
//! git-same status
//!
//! # Also works as git subcommand
//! git same sync
//! ```

pub mod app;
pub mod auth;
pub mod banner;
pub mod cache;
pub mod checks;
pub mod cli;
pub mod commands;
pub mod config;
pub mod discovery;
pub mod domain;
pub mod errors;
pub mod git;
pub mod infra;
pub mod operations;
pub mod output;
pub mod provider;
#[cfg(feature = "tui")]
pub mod setup;
#[cfg(feature = "tui")]
pub mod tui;
pub mod types;
pub mod workflows;

/// Re-export commonly used types for convenience.
pub mod prelude {
    pub use crate::auth::{get_auth, get_auth_for_provider, AuthResult, ResolvedAuthMethod};
    pub use crate::cache::{CacheManager, DiscoveryCache, CACHE_VERSION};
    pub use crate::cli::{Cli, Command, InitArgs, ResetArgs, StatusArgs, SyncCmdArgs};
    pub use crate::config::{
        AuthMethod, Config, ConfigCloneOptions, FilterOptions, ProviderEntry,
        SyncMode as ConfigSyncMode,
    };
    pub use crate::discovery::DiscoveryOrchestrator;
    pub use crate::domain::RepoPathTemplate;
    pub use crate::errors::{AppError, GitError, ProviderError, Result};
    pub use crate::git::{
        CloneOptions, FetchResult, GitOperations, PullResult, RepoStatus, ShellGit,
    };
    pub use crate::operations::clone::{
        CloneManager, CloneManagerOptions, CloneProgress, CloneResult,
    };
    pub use crate::operations::sync::{
        LocalRepo, SyncManager, SyncManagerOptions, SyncMode, SyncResult,
    };
    pub use crate::output::{
        CloneProgressBar, DiscoveryProgressBar, Output, SyncProgressBar, Verbosity,
    };
    pub use crate::provider::{
        create_provider, Credentials, DiscoveryOptions, DiscoveryProgress, NoProgress, Provider,
        RateLimitInfo,
    };
    pub use crate::types::{ActionPlan, OpResult, OpSummary, Org, OwnedRepo, ProviderKind, Repo};
}
