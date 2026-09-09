use super::*;
use crate::macos::monitor_agent::source::{app_main_executable, APP_BUNDLE_ID};

fn render_for(home: &str, bundle: Option<&str>) -> String {
    let paths = MonitorAgentPaths::for_home(Path::new(home));
    let program = paths.helper.clone();
    render(&paths, &program, Path::new(home), bundle).unwrap()
}

/// An app-owned installation execs the bundle's own main executable, so the
/// Full Disk Access grant for "Git-Same" covers the monitor.
fn render_for_app(home: &str, bundle_path: &str) -> String {
    let paths = MonitorAgentPaths::for_home(Path::new(home));
    let program = app_main_executable(Path::new(bundle_path));
    render(&paths, &program, Path::new(home), Some(APP_BUNDLE_ID)).unwrap()
}

// The asserted paths are POSIX: `Path::join` uses backslashes on Windows, so
// the rendered helper path would not match these literals there.
#[cfg(unix)]
#[test]
fn renders_the_managed_helper_invocation() {
    let plist = render_for("/Users/ada", None);
    let helper = "/Users/ada/Library/Application Support/com.zaai.git-same/monitor/git-same";
    assert!(plist.contains("<string>com.zaai.git-same.monitor</string>"));
    assert_eq!(plist.matches(helper).count(), 2, "Program and argv[0]");
    assert!(plist.contains("<string>--foreground</string>\n        <string>--managed</string>"));
    assert!(plist.contains("<string>/Users/ada/Library/Logs/git-same/monitor.log</string>"));
    assert!(plist.contains("<string>/Users/ada/Library/Logs/git-same/monitor.err.log</string>"));
    assert!(!plist.contains("/tmp/"));
    assert!(!plist.contains("__GIT_SAME"));
}

#[test]
fn keep_alive_restarts_only_unsuccessful_exits() {
    let plist = render_for("/Users/ada", None);
    assert!(plist.contains(
        "<key>KeepAlive</key>\n    <dict>\n        <key>SuccessfulExit</key>\n        <false/>"
    ));
    assert!(plist.contains("<key>ThrottleInterval</key>\n    <integer>10</integer>"));
    assert!(plist.contains("<key>RunAtLoad</key>\n    <true/>"));
}

#[cfg(unix)]
#[test]
fn every_substituted_path_is_escaped() {
    let plist = render_for("/Volumes/R&D <x>/Ada \"A\" O'Neil", None);
    assert!(plist.contains("/Volumes/R&amp;D &lt;x&gt;/Ada &quot;A&quot; O&apos;Neil/Library"));
    assert!(!plist.contains("R&D"));
    assert!(!plist.contains("<x>"));
}

#[test]
fn app_association_is_optional() {
    assert!(!render_for("/Users/ada", None).contains("AssociatedBundleIdentifiers"));
    let plist = render_for("/Users/ada", Some("com.zaai.git-same"));
    assert!(plist.contains(
        "<key>AssociatedBundleIdentifiers</key>\n    <array>\n        <string>com.zaai.git-same</string>"
    ));
}

#[test]
fn xml_control_characters_are_rejected() {
    let home = Path::new("/Users/a\u{1}da");
    let paths = MonitorAgentPaths::for_home(home);
    let program = paths.helper.clone();
    assert!(render(&paths, &program, home, None).is_err());
}

#[cfg(unix)]
#[test]
fn non_utf8_paths_are_rejected() {
    use std::os::unix::ffi::OsStrExt;
    let home = Path::new(std::ffi::OsStr::from_bytes(b"/Users/\xff"));
    let paths = MonitorAgentPaths::for_home(home);
    let program = paths.helper.clone();
    assert!(render(&paths, &program, home, None).is_err());
}

#[cfg(target_os = "macos")]
#[test]
fn rendered_plist_passes_plutil_lint() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("agent.plist");
    std::fs::write(
        &path,
        render_for("/Users/R&D/ada", Some("com.zaai.git-same")),
    )
    .unwrap();
    let status = std::process::Command::new("/usr/bin/plutil")
        .arg("-lint")
        .arg(&path)
        .output()
        .unwrap();
    assert!(status.status.success(), "{status:?}");
}

// POSIX literals again: see `renders_the_managed_helper_invocation`.
#[cfg(unix)]
#[test]
fn an_app_owned_agent_execs_the_bundle_executable() {
    let plist = render_for_app("/Users/ada", "/Applications/Git-Same.app");
    let executable = "/Applications/Git-Same.app/Contents/MacOS/git-same-app";

    assert_eq!(plist.matches(executable).count(), 2, "Program and argv[0]");
    // Never the copied helper: that path is a separate TCC identity.
    assert!(!plist.contains("com.zaai.git-same/monitor/git-same<"));
    assert!(plist.contains("<string>--foreground</string>\n        <string>--managed</string>"));
    assert!(plist.contains("com.zaai.git-same"));
}
