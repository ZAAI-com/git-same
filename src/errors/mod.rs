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
//! use git_same::errors::{AppError, Result};
//!
//! fn do_something() -> Result<()> {
//!     Err(AppError::config("missing required field"))
//! }
//! ```

mod app;
mod git;
mod provider;

pub use app::{AppError, Result};
pub use git::GitError;
pub use provider::ProviderError;
