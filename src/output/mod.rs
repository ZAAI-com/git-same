//! Output and progress reporting.

mod printer;
pub mod progress;

pub use printer::{format_count, format_error, format_success, format_warning, Output, Verbosity};
pub use progress::{CloneProgressBar, DiscoveryProgressBar, SyncProgressBar};
