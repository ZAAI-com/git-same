use super::*;
use clap::Parser;
use git_same_cli::cli::Command;

#[test]
fn main_cli_parses_sync_subcommand() {
    let cli = Cli::try_parse_from(["gisa", "sync", "--dry-run", "--pull"]).unwrap();

    match cli.command {
        Some(Command::Sync(args)) => {
            assert!(args.dry_run);
            assert!(args.pull);
        }
        _ => panic!("expected sync subcommand"),
    }
}

#[test]
fn main_cli_without_subcommand_is_none() {
    let cli = Cli::try_parse_from(["gisa"]).unwrap();
    assert!(cli.command.is_none());
}
