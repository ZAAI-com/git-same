//! Error types for the gisa application.
//!
//! This module provides a hierarchy of error types:
//! - [`AppError`] - Top-level application errors
//! - [`ProviderError`] - Errors from Git hosting providers (GitHub, GitLab, etc.)
//! - [`GitError`] - Errors from git command-line operations
//!
//! # Example
//!
//! ```
//! use git_same_core::errors::{AppError, Result};
//!
//! fn do_something() -> Result<()> {
//!     Err(AppError::config("missing required field"))
//! }
//! ```

mod app;
mod git;
mod monitor_agent;
mod provider;

pub use app::{AppError, Result};
pub use git::GitError;
pub use monitor_agent::MonitorAgentError;
pub use provider::ProviderError;
