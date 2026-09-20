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

fn monitor_args(args: &[&str]) -> Result<MonitorArgs, clap::Error> {
    let mut full = vec!["gisa", "monitor"];
    full.extend_from_slice(args);
    Cli::try_parse_from(full).map(|cli| match cli.command {
        Some(Command::Monitor(args)) => args,
        other => panic!("expected monitor, got {other:?}"),
    })
}

#[test]
fn monitor_foreground_forms_still_parse() {
    assert!(!monitor_args(&[]).unwrap().is_control());
    assert!(monitor_args(&["--foreground"]).unwrap().foreground);
    assert_eq!(
        monitor_args(&["--interval", "60"]).unwrap().interval,
        Some(60)
    );
    let daemon = Cli::try_parse_from(["gisa", "daemon"]).unwrap();
    assert!(matches!(daemon.command, Some(Command::Monitor(_))));
}

#[test]
fn monitor_public_controls_parse() {
    assert!(monitor_args(&["--start"]).unwrap().start);
    assert!(monitor_args(&["--stop"]).unwrap().stop);
    assert!(monitor_args(&["--status"]).unwrap().status);
    assert!(monitor_args(&["--uninstall"]).unwrap().uninstall);
    assert!(monitor_args(&["--status"]).unwrap().is_control());
}

#[test]
fn monitor_modes_are_mutually_exclusive() {
    for pair in [
        ["--start", "--stop"],
        ["--status", "--stop"],
        ["--status", "--uninstall"],
        ["--start", "--foreground"],
        ["--uninstall", "--foreground"],
    ] {
        assert!(monitor_args(&pair).is_err(), "{pair:?} must conflict");
    }
}

#[test]
fn monitor_interval_applies_only_to_foreground_runs() {
    assert!(monitor_args(&["--foreground", "--interval", "30"]).is_ok());
    for control in ["--start", "--stop", "--status", "--uninstall"] {
        assert!(
            monitor_args(&[control, "--interval", "30"]).is_err(),
            "{control} must reject --interval"
        );
    }
}

#[test]
fn monitor_private_modes_parse_and_validate() {
    let managed = monitor_args(&["--foreground", "--managed"]).unwrap();
    assert!(managed.managed && managed.is_private() && !managed.is_control());
    assert!(
        monitor_args(&["--managed"]).is_err(),
        "--managed needs --foreground"
    );

    let install = monitor_args(&[
        "--install-agent",
        "--app-path",
        "/Applications/Git-Same.app",
        "--installer-copy",
        "/opt/homebrew/Caskroom/git-same/3.1.2/git-same-service-tool",
    ])
    .unwrap();
    assert!(install.install_agent && install.is_control() && install.is_private());
    assert!(monitor_args(&["--install-agent"]).is_err());
    assert!(monitor_args(&["--install-agent", "--app-path", "/x"]).is_err());

    assert!(
        monitor_args(&["--remove-agent", "--app-path", "/x"])
            .unwrap()
            .remove_agent
    );
    assert!(monitor_args(&["--remove-agent"]).is_err());
    assert!(
        monitor_args(&["--agent-protocol-version"])
            .unwrap()
            .agent_protocol_version
    );
}

#[test]
fn monitor_private_flags_stay_out_of_help_completions_and_manpages() {
    use clap::CommandFactory;
    let mut command = Cli::command();
    let monitor = command.find_subcommand_mut("monitor").unwrap();
    let help = monitor.render_long_help().to_string();
    for hidden in [
        "--managed",
        "--install-agent",
        "--remove-agent",
        "--app-path",
        "--installer-copy",
        "--agent-protocol-version",
    ] {
        assert!(!help.contains(hidden), "{hidden} leaked into help");
    }
    for public in ["--start", "--stop", "--status", "--uninstall"] {
        assert!(help.contains(public), "{public} missing from help");
    }

    #[cfg(feature = "release-tools")]
    {
        let hidden = [
            "--managed",
            "--install-agent",
            "--remove-agent",
            "--app-path",
            "--installer-copy",
            "--agent-protocol-version",
        ];
        let mut completions = Vec::new();
        clap_complete::generate(
            clap_complete::Shell::Bash,
            &mut Cli::command(),
            "git-same",
            &mut completions,
        );
        let completions = String::from_utf8(completions).unwrap();
        let mut manpage = Vec::new();
        clap_mangen::Man::new(Cli::command())
            .render(&mut manpage)
            .unwrap();
        let manpage = String::from_utf8(manpage).unwrap();
        for flag in hidden {
            assert!(
                !completions.contains(flag),
                "{flag} leaked into completions"
            );
            assert!(!manpage.contains(flag), "{flag} leaked into manpage");
        }
    }
}
