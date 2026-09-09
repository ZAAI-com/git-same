use super::*;

#[test]
fn monitor_invocation_matches_first_argument() {
    assert!(is_monitor_invocation(["git-same-app", "monitor"]));
    assert!(is_monitor_invocation([
        "/Applications/Git-Same.app/Contents/MacOS/git-same-app",
        "monitor",
        "--foreground",
    ]));
}

#[test]
fn monitor_invocation_ignores_other_arguments() {
    assert!(!is_monitor_invocation(["git-same-app"]));
    assert!(!is_monitor_invocation([
        "git-same-app",
        "--foreground",
        "monitor"
    ]));
    assert!(!is_monitor_invocation(["git-same-app", "sync"]));
    assert!(!is_monitor_invocation(Vec::<&str>::new()));
}

#[test]
fn managed_flag_selects_the_launchd_startup_rules() {
    // What the rendered plist passes.
    assert!(is_managed([
        "/Applications/Git-Same.app/Contents/MacOS/git-same-app",
        "monitor",
        "--foreground",
        "--managed",
    ]));
    // Started by hand: the user sees failures instead of a silent exit 0.
    assert!(!is_managed(["git-same-app", "monitor", "--foreground"]));
    assert!(!is_managed(["git-same-app", "monitor"]));
}
