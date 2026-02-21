//! Integration adapters around external systems.
//!
//! These modules provide a stable namespace for IO-bound integrations while
//! keeping the existing top-level modules intact during migration.

pub mod auth;
pub mod cache;
pub mod config;
pub mod git;
pub mod output;
pub mod provider;
