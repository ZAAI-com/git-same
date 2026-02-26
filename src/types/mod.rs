//! Core type definitions for gisa.
//!
//! This module contains the fundamental data structures used throughout
//! the application:
//!
//! - [`ProviderKind`] - Identifies Git hosting providers (GitHub, GitLab, etc.)
//! - [`Org`] - Represents an organization
//! - [`Repo`] - Represents a repository
//! - [`OwnedRepo`] - A repository with its owner context
//! - [`ActionPlan`] - Plan for clone/sync operations
//! - [`OpResult`] - Result of a single operation
//! - [`OpSummary`] - Summary statistics for batch operations

mod provider;
mod repo;

pub use provider::ProviderKind;
pub use repo::{ActionPlan, OpResult, OpSummary, Org, OwnedRepo, Repo, SkippedRepo};
