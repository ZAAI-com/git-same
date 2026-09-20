//! Renders the LaunchAgent definition.
//!
//! The template lives in this crate's `assets/` so it ships in the crates.io
//! package. Every substituted value is XML-escaped: home directories may
//! contain spaces, ampersands, or angle brackets.

use super::context::MonitorAgentPaths;
use std::path::Path;

const TEMPLATE: &str = include_str!("../../../assets/com.zaai.git-same.monitor.plist");

/// Renders the plist for `paths`. `associated_bundle` names the app that
/// owns the helper so System Settings attributes the login item to it.
pub fn render(paths: &MonitorAgentPaths, home: &Path, associated_bundle: Option<&str>) -> String {
    let associated = associated_bundle
        .map(|bundle| {
            format!(
                "    <key>AssociatedBundleIdentifiers</key>\n    <array>\n        <string>{}</string>\n    </array>\n",
                escape_xml(bundle)
            )
        })
        .unwrap_or_default();
    TEMPLATE
        .replace("__GIT_SAME_ASSOCIATED_BUNDLE__", &associated)
        .replace("__GIT_SAME_HELPER__", &escape_path(&paths.helper))
        .replace("__GIT_SAME_HOME__", &escape_path(home))
        .replace("__GIT_SAME_STDOUT__", &escape_path(&paths.stdout_log))
        .replace("__GIT_SAME_STDERR__", &escape_path(&paths.stderr_log))
}

fn escape_path(path: &Path) -> String {
    escape_xml(&path.display().to_string())
}

pub fn escape_xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
#[path = "plist_tests.rs"]
mod tests;
