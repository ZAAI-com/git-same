//! CLI argument parsing using clap.
//!
//! This module defines the command-line interface for git-same,
//! including all subcommands and their options.

use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

/// Git-Same - Mirror GitHub structure /orgs/repos/ to local file system
///
/// Available as: git-same, gitsame, gitsa, gisa
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

    /// [deprecated] Clone repositories — use 'gisa sync' instead
    #[command(hide = true)]
    Clone(CloneArgs),

    /// [deprecated] Fetch updates — use 'gisa sync' instead
    #[command(hide = true)]
    Fetch(LegacySyncArgs),

    /// [deprecated] Pull updates — use 'gisa sync --pull' instead
    #[command(hide = true)]
    Pull(LegacySyncArgs),
}

/// Arguments for the init command
#[derive(Args, Debug)]
pub struct InitArgs {
    /// Force overwrite existing config
    #[arg(short, long)]
    pub force: bool,

    /// Path for config file (default: ~/.config/gisa/gisa.config.toml)
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
    /// Workspace path or name to sync
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

/// Arguments for the clone command (deprecated)
#[derive(Args, Debug)]
pub struct CloneArgs {
    /// Base directory for cloned repositories
    pub base_path: PathBuf,

    /// Perform a dry run (show what would be cloned)
    #[arg(short = 'n', long)]
    pub dry_run: bool,

    /// Maximum number of concurrent clones
    #[arg(short, long)]
    pub concurrency: Option<usize>,

    /// Clone depth (0 for full clone)
    #[arg(short = 'd', long)]
    pub depth: Option<u32>,

    /// Clone a specific branch instead of the default
    #[arg(short = 'b', long)]
    pub branch: Option<String>,

    /// Clone submodules recursively
    #[arg(long)]
    pub recurse_submodules: bool,

    /// Include archived repositories
    #[arg(long)]
    pub include_archived: bool,

    /// Include forked repositories
    #[arg(long)]
    pub include_forks: bool,

    /// Filter to specific organizations (can be repeated)
    #[arg(short, long)]
    pub org: Vec<String>,

    /// Exclude specific organizations (can be repeated)
    #[arg(long)]
    pub exclude_org: Vec<String>,

    /// Filter repositories by name pattern (regex)
    #[arg(long)]
    pub filter: Option<String>,

    /// Exclude repositories by name pattern (regex)
    #[arg(long)]
    pub exclude: Option<String>,

    /// Use HTTPS instead of SSH for cloning
    #[arg(long)]
    pub https: bool,

    /// Provider to use (default: all configured)
    #[arg(short, long)]
    pub provider: Option<String>,
}

/// Arguments for the status command
#[derive(Args, Debug)]
pub struct StatusArgs {
    /// Workspace path or name
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
    /// Workspace path or name to set as default (omit to show current)
    pub name: Option<String>,

    /// Clear the default workspace
    #[arg(long)]
    pub clear: bool,
}

/// Arguments for legacy fetch/pull commands (deprecated)
#[derive(Args, Debug)]
pub struct LegacySyncArgs {
    /// Base directory containing cloned repositories
    pub base_path: PathBuf,

    /// Perform a dry run (show what would be synced)
    #[arg(short = 'n', long)]
    pub dry_run: bool,

    /// Maximum number of concurrent operations
    #[arg(short, long)]
    pub concurrency: Option<usize>,

    /// Don't skip repositories with uncommitted changes (sync them anyway)
    #[arg(long)]
    pub no_skip_uncommitted: bool,

    /// Filter to specific organizations (can be repeated)
    #[arg(short, long)]
    pub org: Vec<String>,

    /// Exclude specific organizations (can be repeated)
    #[arg(long)]
    pub exclude_org: Vec<String>,

    /// Filter repositories by name pattern (regex)
    #[arg(long)]
    pub filter: Option<String>,
}

/// Arguments for the reset command
#[derive(Args, Debug)]
pub struct ResetArgs {
    /// Skip confirmation prompt
    #[arg(short, long)]
    pub force: bool,
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
mod tests {
    use super::*;

    #[test]
    fn test_cli_parsing_init() {
        let cli = Cli::try_parse_from(["gisa", "init", "--force"]).unwrap();
        match cli.command {
            Some(Command::Init(args)) => assert!(args.force),
            _ => panic!("Expected Init command"),
        }
    }

    #[test]
    fn test_cli_parsing_setup() {
        let cli = Cli::try_parse_from(["gisa", "setup"]).unwrap();
        match cli.command {
            Some(Command::Setup(args)) => assert!(args.name.is_none()),
            _ => panic!("Expected Setup command"),
        }
    }

    #[test]
    fn test_cli_parsing_setup_with_name() {
        let cli = Cli::try_parse_from(["gisa", "setup", "--name", "work"]).unwrap();
        match cli.command {
            Some(Command::Setup(args)) => assert_eq!(args.name, Some("work".to_string())),
            _ => panic!("Expected Setup command"),
        }
    }

    #[test]
    fn test_cli_parsing_sync() {
        let cli = Cli::try_parse_from(["gisa", "sync", "--pull", "--dry-run"]).unwrap();
        match cli.command {
            Some(Command::Sync(args)) => {
                assert!(args.pull);
                assert!(args.dry_run);
                assert!(args.workspace.is_none());
            }
            _ => panic!("Expected Sync command"),
        }
    }

    #[test]
    fn test_cli_parsing_sync_with_workspace() {
        let cli = Cli::try_parse_from([
            "gisa",
            "sync",
            "--workspace",
            "github",
            "--concurrency",
            "8",
        ])
        .unwrap();
        match cli.command {
            Some(Command::Sync(args)) => {
                assert_eq!(args.workspace, Some("github".to_string()));
                assert_eq!(args.concurrency, Some(8));
            }
            _ => panic!("Expected Sync command"),
        }
    }

    #[test]
    fn test_cli_parsing_status() {
        let cli = Cli::try_parse_from(["gisa", "status", "--uncommitted", "--detailed"]).unwrap();
        match cli.command {
            Some(Command::Status(args)) => {
                assert!(args.uncommitted);
                assert!(args.detailed);
                assert!(args.workspace.is_none());
            }
            _ => panic!("Expected Status command"),
        }
    }

    #[test]
    fn test_cli_parsing_status_with_workspace() {
        let cli = Cli::try_parse_from(["gisa", "status", "--workspace", "work"]).unwrap();
        match cli.command {
            Some(Command::Status(args)) => {
                assert_eq!(args.workspace, Some("work".to_string()));
            }
            _ => panic!("Expected Status command"),
        }
    }

    // Legacy commands still parse (hidden but functional)
    #[test]
    fn test_cli_parsing_legacy_clone() {
        let cli = Cli::try_parse_from(["gisa", "clone", "~/github", "--dry-run"]).unwrap();
        match cli.command {
            Some(Command::Clone(args)) => {
                assert_eq!(args.base_path, PathBuf::from("~/github"));
                assert!(args.dry_run);
            }
            _ => panic!("Expected Clone command"),
        }
    }

    #[test]
    fn test_cli_parsing_legacy_fetch() {
        let cli = Cli::try_parse_from(["gisa", "fetch", "~/github", "--org", "my-org"]).unwrap();
        match cli.command {
            Some(Command::Fetch(args)) => {
                assert_eq!(args.base_path, PathBuf::from("~/github"));
                assert_eq!(args.org, vec!["my-org"]);
            }
            _ => panic!("Expected Fetch command"),
        }
    }

    #[test]
    fn test_cli_parsing_legacy_pull() {
        let cli =
            Cli::try_parse_from(["gisa", "pull", "~/github", "--no-skip-uncommitted"]).unwrap();
        match cli.command {
            Some(Command::Pull(args)) => {
                assert!(args.no_skip_uncommitted);
            }
            _ => panic!("Expected Pull command"),
        }
    }

    #[test]
    fn test_cli_parsing_reset() {
        let cli = Cli::try_parse_from(["gisa", "reset"]).unwrap();
        match cli.command {
            Some(Command::Reset(args)) => assert!(!args.force),
            _ => panic!("Expected Reset command"),
        }
    }

    #[test]
    fn test_cli_parsing_reset_force() {
        let cli = Cli::try_parse_from(["gisa", "reset", "--force"]).unwrap();
        match cli.command {
            Some(Command::Reset(args)) => assert!(args.force),
            _ => panic!("Expected Reset command"),
        }
    }

    #[test]
    fn test_cli_global_flags() {
        let cli = Cli::try_parse_from(["gisa", "-vvv", "--json", "sync"]).unwrap();
        assert_eq!(cli.verbose, 3);
        assert!(cli.json);
        assert_eq!(cli.verbosity(), 3);
    }

    #[test]
    fn test_cli_quiet_flag() {
        let cli = Cli::try_parse_from(["gisa", "--quiet", "sync"]).unwrap();
        assert!(cli.quiet);
        assert!(cli.is_quiet());
        assert_eq!(cli.verbosity(), 0);
    }

    #[test]
    fn test_cli_no_subcommand() {
        let cli = Cli::try_parse_from(["gisa"]).unwrap();
        assert!(cli.command.is_none());
    }

    #[test]
    fn test_cli_parsing_workspace_list() {
        let cli = Cli::try_parse_from(["gisa", "workspace", "list"]).unwrap();
        match cli.command {
            Some(Command::Workspace(args)) => {
                assert!(matches!(args.command, WorkspaceCommand::List));
            }
            _ => panic!("Expected Workspace command"),
        }
    }

    #[test]
    fn test_cli_parsing_workspace_default_set() {
        let cli = Cli::try_parse_from(["gisa", "workspace", "default", "my-ws"]).unwrap();
        match cli.command {
            Some(Command::Workspace(args)) => match args.command {
                WorkspaceCommand::Default(d) => {
                    assert_eq!(d.name, Some("my-ws".to_string()));
                    assert!(!d.clear);
                }
                _ => panic!("Expected Default subcommand"),
            },
            _ => panic!("Expected Workspace command"),
        }
    }

    #[test]
    fn test_cli_parsing_workspace_default_clear() {
        let cli = Cli::try_parse_from(["gisa", "workspace", "default", "--clear"]).unwrap();
        match cli.command {
            Some(Command::Workspace(args)) => match args.command {
                WorkspaceCommand::Default(d) => {
                    assert!(d.clear);
                    assert!(d.name.is_none());
                }
                _ => panic!("Expected Default subcommand"),
            },
            _ => panic!("Expected Workspace command"),
        }
    }

    #[test]
    fn test_cli_parsing_workspace_default_show() {
        let cli = Cli::try_parse_from(["gisa", "workspace", "default"]).unwrap();
        match cli.command {
            Some(Command::Workspace(args)) => match args.command {
                WorkspaceCommand::Default(d) => {
                    assert!(d.name.is_none());
                    assert!(!d.clear);
                }
                _ => panic!("Expected Default subcommand"),
            },
            _ => panic!("Expected Workspace command"),
        }
    }

    #[test]
    fn verify_cli() {
        use clap::CommandFactory;
        Cli::command().debug_assert();
    }
}
