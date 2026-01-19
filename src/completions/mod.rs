//! Shell completion generation module.
//!
//! This module provides shell completion generation for gisa.
//! Completions are generated using clap_complete and can be output
//! for various shells.
//!
//! # Example
//!
//! ```no_run
//! use gisa::completions::{generate_completions, ShellType};
//!
//! // Generate bash completions (prints to stdout)
//! generate_completions(ShellType::Bash);
//! ```
//!
//! # Installation
//!
//! ## Bash
//!
//! ```bash
//! gisa completions bash > ~/.local/share/bash-completion/completions/gisa
//! ```
//!
//! ## Zsh
//!
//! ```bash
//! gisa completions zsh > ~/.zfunc/_gisa
//! ```
//!
//! ## Fish
//!
//! ```bash
//! gisa completions fish > ~/.config/fish/completions/gisa.fish
//! ```

pub use crate::cli::{generate_completions, ShellType};
