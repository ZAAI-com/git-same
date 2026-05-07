//! # git-same-cli — git-same CLI + TUI
//!
//! Library scaffolding for the `git-same` binary plus the release-tools
//! helpers `gen-completions` and `gen-manpage`. Implementation detail of
//! the binary; engine logic lives in `git-same-core`.

pub mod app;
pub mod banner;
pub mod cli;
pub mod commands;
#[cfg(feature = "tui")]
pub mod setup;
#[cfg(feature = "tui")]
pub mod tui;
