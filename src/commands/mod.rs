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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::output::{Output, Verbosity};

    fn quiet_output() -> Output {
        Output::new(Verbosity::Quiet, false)
    }

    #[test]
    fn test_concurrency_within_limit() {
        let output = quiet_output();
        assert_eq!(warn_if_concurrency_capped(4, &output), 4);
    }

    #[test]
    fn test_concurrency_at_limit() {
        let output = quiet_output();
        assert_eq!(
            warn_if_concurrency_capped(MAX_CONCURRENCY, &output),
            MAX_CONCURRENCY
        );
    }

    #[test]
    fn test_concurrency_above_limit() {
        let output = quiet_output();
        assert_eq!(
            warn_if_concurrency_capped(MAX_CONCURRENCY + 10, &output),
            MAX_CONCURRENCY
        );
    }

    #[test]
    fn test_expand_path_absolute() {
        let path = Path::new("/tmp/some/path");
        assert_eq!(expand_path(path), PathBuf::from("/tmp/some/path"));
    }

    #[test]
    fn test_expand_path_tilde() {
        let path = Path::new("~/foo");
        let expanded = expand_path(path);
        assert!(!expanded.to_string_lossy().contains('~'));
        assert!(expanded.to_string_lossy().ends_with("/foo"));
    }
}
