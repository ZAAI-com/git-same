use super::*;
use crate::cli::Cli;
use clap::Parser;

fn timing(args: &[&str]) -> HookTiming {
    let cli = Cli::try_parse_from(args).expect("valid arguments");
    hook_timing(&cli.command.expect("subcommand"), cli.config.is_some())
}

#[test]
fn sync_ensures_first_but_dry_runs_change_nothing() {
    assert_eq!(timing(&["gisa", "sync"]), HookTiming::Before);
    assert_eq!(timing(&["gisa", "sync", "--dry-run"]), HookTiming::None);
}

#[test]
fn refresh_ensures_first() {
    assert_eq!(timing(&["gisa", "refresh"]), HookTiming::Before);
}

#[test]
fn registry_changes_ensure_afterwards_and_refresh() {
    assert_eq!(timing(&["gisa", "init"]), HookTiming::AfterThenRefresh);
    assert_eq!(
        timing(&["gisa", "scan", "--register"]),
        HookTiming::AfterThenRefresh
    );
}

#[cfg(feature = "tui")]
#[test]
fn setup_wizard_ensures_afterwards() {
    assert_eq!(timing(&["gisa", "setup"]), HookTiming::AfterThenRefresh);
}

#[test]
fn read_only_and_custom_targets_change_nothing() {
    assert_eq!(timing(&["gisa", "scan"]), HookTiming::None);
    assert_eq!(
        timing(&["gisa", "init", "--path", "/tmp/x.toml"]),
        HookTiming::None
    );
    assert_eq!(timing(&["gisa", "status"]), HookTiming::None);
    assert_eq!(timing(&["gisa", "workspace", "list"]), HookTiming::None);
    assert_eq!(timing(&["gisa", "reset", "--force"]), HookTiming::None);
}

#[test]
fn monitor_invocations_never_ensure_recursively() {
    for args in [
        vec!["gisa", "monitor"],
        vec!["gisa", "monitor", "--foreground", "--managed"],
        vec!["gisa", "monitor", "--status"],
        vec!["gisa", "monitor", "--stop"],
        vec!["gisa", "daemon"],
    ] {
        assert_eq!(timing(&args), HookTiming::None, "{args:?}");
    }
}

#[test]
fn configuration_override_disables_every_hook() {
    for args in [
        vec!["gisa", "-C", "/tmp/c.toml", "sync"],
        vec!["gisa", "-C", "/tmp/c.toml", "refresh"],
        vec!["gisa", "-C", "/tmp/c.toml", "scan", "--register"],
    ] {
        assert_eq!(timing(&args), HookTiming::None, "{args:?}");
    }
}
