//! Long-running monitor that powers Finder badge updates.
//!
//! The monitor periodically scans configured workspaces, computes badge
//! state, atomically writes `status.json`, and serves a Unix socket so the
//! macOS Finder Sync extension (or other clients) can request refreshes.
//!
//! This module is the single source of truth for the loop. It is consumed
//! by the `gisa monitor` CLI subcommand and is also reusable by a host
//! application (e.g. the Tauri app) that wants to run the monitor in-process
//! instead of as a separate child process. Callers supply a shutdown future
//! so they can wire whichever termination signal makes sense for them
//! (`ctrl_c` + SIGTERM for the CLI, a `tokio::sync::Notify` for a host).

pub mod incremental;
pub mod live_config;
pub mod managed;
pub mod owner_classifier;
pub mod process;
pub mod run;
pub mod runtime_guard;
#[cfg(unix)]
pub mod socket_handler;

pub use managed::run_managed;
pub use run::{default_shutdown_signal, run, run_with, Options, RunContext};
pub use runtime_guard::MonitorMode;
