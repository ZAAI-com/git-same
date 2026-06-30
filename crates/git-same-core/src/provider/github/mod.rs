//! GitHub provider implementation.
//!
//! Supports both github.com and GitHub Enterprise Server.

mod client;
mod pagination;

pub use client::GitHubProvider;

/// Default GitHub API URL
pub const GITHUB_API_URL: &str = "https://api.github.com";
