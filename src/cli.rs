//! CLI argument parsing using clap.
//!
//! This module defines the command-line interface for git-same,
//! including all subcommands and their options.

use clap::{Args, Parser, Subcommand, ValueEnum};
use clap_complete::Shell;
use std::path::PathBuf;

/// Git-Same - Mirror GitHub org/repo structure locally
///
/// Discovers all GitHub organizations and repositories you have access to,
/// then clones/syncs them to maintain a local mirror of your org structure.
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
    pub command: Command,
}

/// Git-Same subcommands
#[derive(Subcommand, Debug)]
pub enum Command {
    /// Initialize git-same configuration
    Init(InitArgs),

    /// Clone repositories to local filesystem
    Clone(CloneArgs),

    /// Fetch updates from remotes (doesn't modify working tree)
    Fetch(SyncArgs),

    /// Pull updates from remotes (modifies working tree)
    Pull(SyncArgs),

    /// Show status of local repositories
    Status(StatusArgs),

    /// Generate shell completions
    Completions(CompletionsArgs),
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

/// Arguments for the clone command
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

    /// Force re-discovery (ignore cache)
    #[arg(long)]
    pub refresh: bool,

    /// Skip using cache entirely
    #[arg(long)]
    pub no_cache: bool,
}

/// Arguments for fetch and pull commands
#[derive(Args, Debug)]
pub struct SyncArgs {
    /// Base directory containing cloned repositories
    pub base_path: PathBuf,

    /// Perform a dry run (show what would be synced)
    #[arg(short = 'n', long)]
    pub dry_run: bool,

    /// Maximum number of concurrent operations
    #[arg(short, long)]
    pub concurrency: Option<usize>,

    /// Skip repositories with uncommitted changes
    #[arg(long, default_value_t = true)]
    pub skip_dirty: bool,

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

/// Arguments for the status command
#[derive(Args, Debug)]
pub struct StatusArgs {
    /// Base directory containing cloned repositories
    pub base_path: PathBuf,

    /// Show only repositories with changes
    #[arg(short, long)]
    pub dirty: bool,

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

/// Arguments for the completions command
#[derive(Args, Debug)]
pub struct CompletionsArgs {
    /// Shell to generate completions for
    #[arg(value_enum)]
    pub shell: ShellType,
}

/// Supported shells for completions
#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellType {
    Bash,
    Zsh,
    Fish,
    PowerShell,
    Elvish,
}

impl From<ShellType> for Shell {
    fn from(shell: ShellType) -> Self {
        match shell {
            ShellType::Bash => Shell::Bash,
            ShellType::Zsh => Shell::Zsh,
            ShellType::Fish => Shell::Fish,
            ShellType::PowerShell => Shell::PowerShell,
            ShellType::Elvish => Shell::Elvish,
        }
    }
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

/// Generate shell completions.
pub fn generate_completions(shell: ShellType) {
    use clap::CommandFactory;
    use clap_complete::generate;
    use std::io;

    let mut cmd = Cli::command();
    let shell: Shell = shell.into();
    generate(shell, &mut cmd, "gisa", &mut io::stdout());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_parsing_clone() {
        let cli = Cli::try_parse_from([
            "gisa",
            "clone",
            "~/github",
            "--dry-run",
            "--concurrency",
            "8",
        ])
        .unwrap();

        match cli.command {
            Command::Clone(args) => {
                assert_eq!(args.base_path, PathBuf::from("~/github"));
                assert!(args.dry_run);
                assert_eq!(args.concurrency, Some(8));
            }
            _ => panic!("Expected Clone command"),
        }
    }

    #[test]
    fn test_cli_parsing_fetch() {
        let cli = Cli::try_parse_from(["gisa", "fetch", "~/github", "--org", "my-org"]).unwrap();

        match cli.command {
            Command::Fetch(args) => {
                assert_eq!(args.base_path, PathBuf::from("~/github"));
                assert_eq!(args.org, vec!["my-org"]);
            }
            _ => panic!("Expected Fetch command"),
        }
    }

    #[test]
    fn test_cli_parsing_pull() {
        let cli = Cli::try_parse_from(["gisa", "pull", "~/github", "--skip-dirty"]).unwrap();

        match cli.command {
            Command::Pull(args) => {
                assert!(args.skip_dirty);
            }
            _ => panic!("Expected Pull command"),
        }
    }

    #[test]
    fn test_cli_parsing_status() {
        let cli =
            Cli::try_parse_from(["gisa", "status", "~/github", "--dirty", "--detailed"]).unwrap();

        match cli.command {
            Command::Status(args) => {
                assert!(args.dirty);
                assert!(args.detailed);
            }
            _ => panic!("Expected Status command"),
        }
    }

    #[test]
    fn test_cli_parsing_init() {
        let cli = Cli::try_parse_from(["gisa", "init", "--force"]).unwrap();

        match cli.command {
            Command::Init(args) => {
                assert!(args.force);
            }
            _ => panic!("Expected Init command"),
        }
    }

    #[test]
    fn test_cli_parsing_completions() {
        let cli = Cli::try_parse_from(["gisa", "completions", "bash"]).unwrap();

        match cli.command {
            Command::Completions(args) => {
                assert_eq!(args.shell, ShellType::Bash);
            }
            _ => panic!("Expected Completions command"),
        }
    }

    #[test]
    fn test_cli_global_flags() {
        let cli = Cli::try_parse_from(["gisa", "-vvv", "--json", "clone", "~/github"]).unwrap();

        assert_eq!(cli.verbose, 3);
        assert!(cli.json);
        assert_eq!(cli.verbosity(), 3);
    }

    #[test]
    fn test_cli_quiet_flag() {
        let cli = Cli::try_parse_from(["gisa", "--quiet", "clone", "~/github"]).unwrap();

        assert!(cli.quiet);
        assert!(cli.is_quiet());
        assert_eq!(cli.verbosity(), 0);
    }

    #[test]
    fn test_cli_clone_with_filters() {
        let cli = Cli::try_parse_from([
            "gisa",
            "clone",
            "~/github",
            "--org",
            "org1",
            "--org",
            "org2",
            "--exclude-org",
            "skip-this",
            "--include-archived",
            "--include-forks",
        ])
        .unwrap();

        match cli.command {
            Command::Clone(args) => {
                assert_eq!(args.org, vec!["org1", "org2"]);
                assert_eq!(args.exclude_org, vec!["skip-this"]);
                assert!(args.include_archived);
                assert!(args.include_forks);
            }
            _ => panic!("Expected Clone command"),
        }
    }

    #[test]
    fn test_cli_clone_https_flag() {
        let cli = Cli::try_parse_from(["gisa", "clone", "~/github", "--https"]).unwrap();

        match cli.command {
            Command::Clone(args) => {
                assert!(args.https);
            }
            _ => panic!("Expected Clone command"),
        }
    }

    #[test]
    fn test_shell_type_conversion() {
        assert_eq!(Shell::from(ShellType::Bash), Shell::Bash);
        assert_eq!(Shell::from(ShellType::Zsh), Shell::Zsh);
        assert_eq!(Shell::from(ShellType::Fish), Shell::Fish);
        assert_eq!(Shell::from(ShellType::PowerShell), Shell::PowerShell);
        assert_eq!(Shell::from(ShellType::Elvish), Shell::Elvish);
    }

    #[test]
    fn verify_cli() {
        // This verifies the CLI definition is valid
        use clap::CommandFactory;
        Cli::command().debug_assert();
    }
}
