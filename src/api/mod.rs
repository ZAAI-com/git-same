//! Public API layer for repository status scanning.
//!
//! This module exposes the `RepoScanService` — the core service used by
//! the daemon, CLI, and any future frontend (HTTP server, native app) to
//! scan repositories and compute badge status.
//!
//! ## Architecture
//!
//! The service sits between:
//! - **Consumers** (daemon loop, CLI `status` command, socket REFRESH handler)
//! - **Implementations** (`GitOperations` trait for git, `Config` for workspace layout)
//!
//! Consumers hold a `&RepoScanService` and call `scan_all()`, `scan_workspace()`,
//! or `scan_repo()` to get structured `FinderStatus` / `FinderRepoStatus` values.

pub mod service;

pub use service::RepoScanService;
