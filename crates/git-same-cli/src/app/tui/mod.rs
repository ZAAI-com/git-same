//! TUI-facing adapters.

#[cfg(feature = "tui")]
pub use crate::setup::run_setup;
#[cfg(feature = "tui")]
pub use crate::tui::run_tui;
