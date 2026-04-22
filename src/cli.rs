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
#[command(version, about)]
#[command(propagate_version = true)]
#[command(
    long_about = "Mirror GitHub structure /orgs/repos/ to local file system.\n\n\
        Git-Same discovers your GitHub organization and user repository structures \
        and mirrors them locally. It creates a directory tree matching /org/repo/ \
        layout, clones new repositories in parallel, and keeps existing clones \
        in sync.\n\n\
        Run without a subcommand to launch the interactive TUI.\n\
        Config: ~/.config/git-same/config.toml\n\
        Auth: uses `gh auth token` (GitHub CLI required)",
    after_help = "\
Examples (quick-start):
  gisa init                  Create default config
  gisa setup                 Interactive workspace wizard
  gisa sync                  Clone new + fetch existing repos
  gisa status                See which repos have changes

  gisa                       Launch interactive TUI

Aliases: git-same, gitsame, gitsa, gisa
Also works as: git same (git subcommand)"
)]
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
    #[command(
        long_about = "Initialize git-same configuration.\n\n\
            Creates the config file at the default location \
            (~/.config/git-same/config.toml) or a custom path. This is the first \
            step before configuring workspaces. If a config file already exists, \
            use --force to overwrite it.",
        after_help = "\
Examples:
  gisa init                  Create config at default location
  gisa init --force          Overwrite existing config
  gisa init --path ./my.toml Create config at a custom path"
    )]
    Init(InitArgs),

    /// Configure a workspace (interactive wizard)
    #[command(
        long_about = "Launch an interactive wizard to configure a new workspace. \
            The wizard prompts for a base directory, GitHub organizations or users \
            to mirror, clone method (SSH/HTTPS), and filter rules. Run this after \
            `gisa init` to set up your first workspace, or again to add more.",
        after_help = "\
Examples:
  gisa setup                 Start the interactive wizard
  gisa setup --name work     Pre-set the workspace name to 'work'"
    )]
    Setup(SetupArgs),

    /// Sync repositories (discover, clone new, fetch/pull existing)
    #[command(
        long_about = "Discover repositories from configured GitHub organizations, \
            clone any new repositories, and fetch (or pull) existing ones. Discovery \
            results are cached; use --refresh to force re-discovery. Repositories \
            with uncommitted changes are skipped by default to avoid conflicts.",
        after_help = "\
Examples:
  gisa sync                         Sync the default workspace
  gisa sync -w work                 Sync a specific workspace
  gisa sync --pull                  Pull instead of fetch
  gisa sync --dry-run               Preview what would happen
  gisa sync --concurrency 8         Use 8 parallel operations
  gisa sync --refresh               Ignore cache, re-discover repos
  gisa sync --no-skip-uncommitted   Don't skip dirty repos"
    )]
    Sync(SyncCmdArgs),

    /// Show status of local repositories
    #[command(
        long_about = "Scan local repositories in a workspace and report their git \
            status. Shows which repos have uncommitted changes, which are behind \
            upstream, and provides an overview of workspace health. Useful before \
            and after syncing.",
        after_help = "\
Examples:
  gisa status                       Status of default workspace
  gisa status -w work               Status of a specific workspace
  gisa status --uncommitted         Show only repos with changes
  gisa status --behind              Show only repos behind upstream
  gisa status --detailed            Verbose per-repo status
  gisa status --org my-org          Filter to one organization"
    )]
    Status(StatusArgs),

    /// Manage workspaces (list, set default)
    #[command(
        long_about = "Manage git-same workspaces. A workspace maps a local \
            directory to one or more GitHub organizations or users. Use \
            subcommands to list all configured workspaces or get/set the default.",
        after_help = "\
Examples:
  gisa workspace list               List all workspaces
  gisa workspace default            Show the current default
  gisa workspace default my-ws      Set 'my-ws' as default
  gisa workspace default --clear    Clear the default"
    )]
    Workspace(WorkspaceArgs),

    /// Reset gisa — remove all config, workspaces, and cache
    #[command(
        long_about = "Remove all git-same configuration, workspace metadata, and \
            cached discovery data. Cloned repositories on disk are NOT deleted — \
            only git-same's own config and cache files are removed. You will be \
            prompted for confirmation unless --force is used.",
        after_help = "\
Examples:
  gisa reset                 Reset with confirmation prompt
  gisa reset --force         Reset without confirmation"
    )]
    Reset(ResetArgs),

    /// Run background daemon for Finder/file manager extension
    #[command(
        long_about = "Run a background daemon that monitors workspace repositories and \
            writes status data for the macOS Finder extension (or other file manager \
            plugins). The daemon periodically scans repos, computes badge colors, and \
            writes status to ~/.config/git-same/finder/status.json. It also listens \
            on a Unix socket for refresh requests from the extension.",
        after_help = "\
Examples:
  gisa daemon                      Start daemon (daemonizes by default)
  gisa daemon --foreground         Run in foreground (useful for debugging)
  gisa daemon --interval 60        Poll every 60 seconds
  gisa daemon --status             Check if daemon is running
  gisa daemon --stop               Stop a running daemon"
    )]
    Daemon(DaemonArgs),

    /// Scan a directory tree for unregistered workspaces (.git-same/ folders)
    #[command(
        long_about = "Walk a directory tree looking for .git-same/ marker folders \
            that indicate existing workspace roots not yet registered in your \
            config. Useful when you have repos organized in org/repo layout and \
            want git-same to adopt them. Use --register to automatically add \
            discovered workspaces.",
        after_help = "\
Examples:
  gisa scan                         Scan current directory
  gisa scan ~/projects              Scan a specific directory
  gisa scan --depth 3               Limit search depth
  gisa scan ~/projects --register   Auto-register found workspaces"
    )]
    Scan(ScanArgs),

    /// Ask the running daemon to refresh status.json immediately
    #[command(
        long_about = "Send a refresh request to the background daemon so it \
            rewrites ~/.config/git-same/finder/status.json right now. Useful \
            after manually deleting a repo, or when debugging Finder badges. \
            Fails with a clear error if the daemon is not running.",
        after_help = "\
Examples:
  gisa refresh                      Refresh everything the daemon knows about
  gisa refresh --path ~/work/org    Refresh a single folder"
    )]
    Refresh(RefreshArgs),
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
    #[command(
        long_about = "Display all configured workspaces with their base paths, \
            associated organizations, and which one is set as the default."
    )]
    List,
    /// Set or show the default workspace
    #[command(
        long_about = "Set or display the default workspace. The default workspace \
            is used by commands like `sync` and `status` when no --workspace flag \
            is given. Pass a workspace name to set it, use --clear to unset, or \
            omit arguments to show the current default.",
        after_help = "\
Examples:
  gisa workspace default             Show current default
  gisa workspace default work        Set 'work' as default
  gisa workspace default --clear     Clear the default"
    )]
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

/// Arguments for the daemon command
#[derive(Args, Debug)]
pub struct DaemonArgs {
    /// Run in foreground instead of daemonizing
    #[arg(long)]
    pub foreground: bool,

    /// Polling interval in seconds
    #[arg(long, default_value = "30")]
    pub interval: u64,

    /// Stop a running daemon
    #[arg(long)]
    pub stop: bool,

    /// Show daemon status (running, PID, last scan)
    #[arg(long)]
    pub status: bool,
}

/// Arguments for the refresh command
#[derive(Args, Debug)]
pub struct RefreshArgs {
    /// Refresh a specific folder instead of everything the daemon monitors
    #[arg(long)]
    pub path: Option<PathBuf>,
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
