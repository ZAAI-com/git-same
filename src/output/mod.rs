//! Output and progress reporting module.
//!
//! This module provides utilities for consistent output formatting
//! and progress reporting using indicatif.
//!
//! # Example
//!
//! ```no_run
//! use gisa::output::{Output, Verbosity, CloneProgressBar};
//!
//! // Create output handler
//! let output = Output::new(Verbosity::Normal, false);
//! output.info("Starting operation...");
//! output.success("Operation completed");
//!
//! // Create progress bar for clone operations
//! let progress = CloneProgressBar::new(10, Verbosity::Normal);
//! // ... perform cloning operations
//! progress.finish(8, 1, 1);
//! ```

pub mod progress;

pub use progress::{
    format_count, format_error, format_success, format_warning, CloneProgressBar,
    DiscoveryProgressBar, Output, SyncProgressBar, Verbosity,
};
