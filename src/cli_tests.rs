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

#[test]
fn test_cli_rejects_clone_subcommand() {
    let cli = Cli::try_parse_from(["gisa", "clone"]);
    assert!(cli.is_err());
}

#[test]
fn test_cli_rejects_fetch_subcommand() {
    let cli = Cli::try_parse_from(["gisa", "fetch"]);
    assert!(cli.is_err());
}

#[test]
fn test_cli_rejects_pull_subcommand() {
    let cli = Cli::try_parse_from(["gisa", "pull"]);
    assert!(cli.is_err());
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
