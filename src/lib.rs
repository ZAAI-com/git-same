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
//! The tool can be invoked using any of these names:
//! - `git-same` (main command)
//! - `git same` (as a git subcommand)
//! - `gitsame`
//! - `gitsa`
//! - `gisa`
//!
//! ## Example
//!
//! ```bash
//! # Initialize configuration
//! git-same init
//!
//! # Clone all repositories (dry run first)
//! git-same clone ~/github --dry-run
//!
//! # Clone for real
//! git-same clone ~/github
//!
//! # Fetch updates
//! git-same fetch ~/github
//!
//! # Pull updates (modifies working tree)
//! git-same pull ~/github
//!
//! # Show status
//! git-same status ~/github
//!
//! # Also works as git subcommand
//! git same clone ~/github
//! ```

pub mod auth;
pub mod cli;
pub mod clone;
pub mod completions;
pub mod config;
pub mod discovery;
pub mod errors;
pub mod git;
pub mod output;
pub mod provider;
pub mod sync;
pub mod types;

/// Re-export commonly used types for convenience.
pub mod prelude {
    pub use crate::auth::{get_auth, get_auth_for_provider, AuthResult, ResolvedAuthMethod};
    pub use crate::cli::{Cli, CloneArgs, Command, InitArgs, StatusArgs, SyncArgs};
    pub use crate::clone::{CloneManager, CloneManagerOptions, CloneProgress, CloneResult};
    pub use crate::completions::{generate_completions, ShellType};
    pub use crate::config::{
        AuthMethod, CloneOptions as ConfigCloneOptions, Config, FilterOptions, ProviderEntry,
        SyncMode as ConfigSyncMode,
    };
    pub use crate::discovery::DiscoveryOrchestrator;
    pub use crate::errors::{AppError, GitError, ProviderError, Result};
    pub use crate::git::{
        CloneOptions, FetchResult, GitOperations, PullResult, RepoStatus, ShellGit,
    };
    pub use crate::output::{
        CloneProgressBar, DiscoveryProgressBar, Output, SyncProgressBar, Verbosity,
    };
    pub use crate::provider::{
        create_provider, Credentials, DiscoveryOptions, DiscoveryProgress, NoProgress, Provider,
        RateLimitInfo,
    };
    pub use crate::sync::{LocalRepo, SyncManager, SyncManagerOptions, SyncMode, SyncResult};
    pub use crate::types::{ActionPlan, OpResult, OpSummary, Org, OwnedRepo, ProviderKind, Repo};
}
