//! # git-same-core
//!
//! Engine for git-same: discovery, clone/sync orchestration, IPC, status.
//! See the `git-same` (CLI) crate for the user-facing binary.

pub mod api;
pub mod auth;
pub mod cache;
pub mod checks;
pub mod config;
pub mod discovery;
pub mod domain;
pub mod errors;
pub mod git;
pub mod infra;
pub mod ipc;
pub mod monitor;
pub mod operations;
pub mod output;
pub mod provider;
pub mod types;
pub mod workflows;

/// Re-export commonly used types for convenience.
pub mod prelude {
    pub use crate::auth::{get_auth, get_auth_for_provider, AuthResult};
    pub use crate::cache::{CacheManager, DiscoveryCache, SyncHistoryManager, CACHE_VERSION};
    pub use crate::config::{
        Config, ConfigCloneOptions, FilterOptions, SyncMode as ConfigSyncMode, WorkspaceConfig,
        WorkspaceProvider,
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
    pub use crate::types::{
        ActionPlan, OpResult, OpSummary, Org, OwnedRepo, ProviderKind, Repo, RepoEntry,
        SyncHistoryEntry,
    };
}

#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;
