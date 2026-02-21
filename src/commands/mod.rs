//! Command handlers for the CLI subcommands.
//!
//! This module contains the runtime behavior for each subcommand,
//! separated from `main.rs` so the entrypoint stays focused on bootstrapping.

pub mod clone;
pub mod init;
pub mod status;
pub mod sync;

pub use clone::run as run_clone;
pub use init::run as run_init;
pub use status::run as run_status;
pub use sync::run as run_sync;

use crate::adapters::output::Output;
use crate::core::operations::clone::MAX_CONCURRENCY;
use std::path::{Path, PathBuf};

/// Warn if requested concurrency exceeds the maximum.
/// Returns the effective concurrency to use.
pub(crate) fn warn_if_concurrency_capped(requested: usize, output: &Output) -> usize {
    if requested > MAX_CONCURRENCY {
        output.warn(&format!(
            "Requested concurrency {} exceeds maximum {}. Using {} instead.",
            requested, MAX_CONCURRENCY, MAX_CONCURRENCY
        ));
        MAX_CONCURRENCY
    } else {
        requested
    }
}

/// Expands ~ in a path.
pub(crate) fn expand_path(path: &Path) -> PathBuf {
    let path_str = path.to_string_lossy();
    let expanded = shellexpand::tilde(&path_str);
    PathBuf::from(expanded.as_ref())
}
