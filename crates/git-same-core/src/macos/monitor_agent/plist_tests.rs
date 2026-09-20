use super::*;

fn render_for(home: &str, bundle: Option<&str>) -> String {
    let home = Path::new(home);
    render(&MonitorAgentPaths::for_home(home), home, bundle)
}

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
