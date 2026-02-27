//! CLI argument parsing using clap.
//!
//! This module defines the command-line interface for git-same,
//! including all subcommands and their options.

use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

/// Git-Same - Mirror GitHub structure /orgs/repos/ to local file system
///
/// Available as: git-same (primary), gitsame, gitsa, gisa (symlink aliases)
/// Alias list: see toolkit/packaging/binary-aliases.txt
/// Also works as: git same (git subcommand)
#[derive(Parser, Debug)]
#[command(name = "git-same")]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    /// Increase verbosity (-v, -vv, -vvv)
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    pub verbose: u8,

    /// Suppress all output except errors
    #[arg(short, long, global = true)]
    pub quiet: bool,

    /// Output in JSON format
    #[arg(long, global = true)]
    pub json: bool,

    /// Path to config file
    #[arg(short = 'C', long, global = true)]
    pub config: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Option<Command>,
}

/// Git-Same subcommands
#[derive(Subcommand, Debug)]
pub enum Command {
    /// Initialize git-same configuration
    Init(InitArgs),

    /// Configure a workspace (interactive wizard)
    Setup(SetupArgs),

    /// Sync repositories (discover, clone new, fetch/pull existing)
    Sync(SyncCmdArgs),

    /// Show status of local repositories
    Status(StatusArgs),

    /// Manage workspaces (list, set default)
    Workspace(WorkspaceArgs),

    /// Reset gisa — remove all config, workspaces, and cache
    Reset(ResetArgs),

    /// Scan a directory tree for unregistered workspaces (.git-same/ folders)
    Scan(ScanArgs),
}

/// Arguments for the init command
#[derive(Args, Debug)]
pub struct InitArgs {
    /// Force overwrite existing config
    #[arg(short, long)]
    pub force: bool,

    /// Path for config file (default: ~/.config/git-same/config.toml)
    #[arg(short, long)]
    pub path: Option<PathBuf>,
}

/// Arguments for the setup command
#[derive(Args, Debug)]
pub struct SetupArgs {
    /// Workspace name (auto-derived from base path if omitted)
    #[arg(short, long)]
    pub name: Option<String>,
}

/// Arguments for the sync command
#[derive(Args, Debug)]
pub struct SyncCmdArgs {
    /// Workspace path or folder name to sync
    #[arg(short, long)]
    pub workspace: Option<String>,

    /// Use pull instead of fetch for existing repos
    #[arg(long)]
    pub pull: bool,

    /// Perform a dry run (show what would be done)
    #[arg(short = 'n', long)]
    pub dry_run: bool,

    /// Maximum number of concurrent operations
    #[arg(short, long)]
    pub concurrency: Option<usize>,

    /// Force re-discovery (ignore cache)
    #[arg(long)]
    pub refresh: bool,

    /// Don't skip repositories with uncommitted changes
    #[arg(long)]
    pub no_skip_uncommitted: bool,
}

/// Arguments for the status command
#[derive(Args, Debug)]
pub struct StatusArgs {
    /// Workspace path or folder name
    #[arg(short, long)]
    pub workspace: Option<String>,

    /// Show only repositories with changes
    #[arg(short = 'd', long)]
    pub uncommitted: bool,

    /// Show only repositories behind upstream
    #[arg(short, long)]
    pub behind: bool,

    /// Show detailed status for each repository
    #[arg(long)]
    pub detailed: bool,

    /// Filter to specific organizations (can be repeated)
    #[arg(short, long)]
    pub org: Vec<String>,
}

/// Arguments for the workspace command
#[derive(Args, Debug)]
pub struct WorkspaceArgs {
    #[command(subcommand)]
    pub command: WorkspaceCommand,
}

/// Workspace subcommands
#[derive(Subcommand, Debug)]
pub enum WorkspaceCommand {
    /// List configured workspaces
    List,
    /// Set or show the default workspace
    Default(WorkspaceDefaultArgs),
}

/// Arguments for the workspace default subcommand
#[derive(Args, Debug)]
pub struct WorkspaceDefaultArgs {
    /// Workspace path or folder name to set as default (omit to show current)
    #[arg(value_name = "WORKSPACE")]
    pub name: Option<String>,

    /// Clear the default workspace
    #[arg(long)]
    pub clear: bool,
}

/// Arguments for the reset command
#[derive(Args, Debug)]
pub struct ResetArgs {
    /// Skip confirmation prompt
    #[arg(short, long)]
    pub force: bool,
}

/// Arguments for the scan command
#[derive(Args, Debug)]
pub struct ScanArgs {
    /// Root directory to scan (default: current directory)
    pub path: Option<PathBuf>,

    /// Maximum directory depth to search (default: 5)
    #[arg(short, long, default_value = "5")]
    pub depth: usize,

    /// Register discovered workspaces automatically
    #[arg(long)]
    pub register: bool,
}

impl Cli {
    /// Parse command line arguments.
    pub fn parse_args() -> Self {
        Self::parse()
    }

    /// Get the effective verbosity level (0-3).
    pub fn verbosity(&self) -> u8 {
        if self.quiet {
            0
        } else {
            self.verbose.min(3)
        }
    }

    /// Check if output should be suppressed.
    pub fn is_quiet(&self) -> bool {
        self.quiet
    }

    /// Check if JSON output is requested.
    pub fn is_json(&self) -> bool {
        self.json
    }
}

#[cfg(test)]
#[path = "cli_tests.rs"]
mod tests;
