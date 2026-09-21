//! Renders the LaunchAgent definition.
//!
//! The template lives in this crate's `assets/` so it ships in the crates.io
//! package. Every substituted value is XML-escaped: home directories may
//! contain spaces, ampersands, or angle brackets.

use super::context::MonitorAgentPaths;
use crate::errors::MonitorAgentError;
use std::path::Path;

const TEMPLATE: &str = include_str!("../../../assets/com.zaai.git-same.monitor.plist");

/// Renders the plist for `paths`. `associated_bundle` names the app that
/// owns the helper so System Settings attributes the login item to it.
pub fn render(
    paths: &MonitorAgentPaths,
    home: &Path,
    associated_bundle: Option<&str>,
) -> Result<String, MonitorAgentError> {
    let associated = associated_bundle
        .map(|bundle| -> Result<_, MonitorAgentError> {
            validate_xml(bundle)?;
            Ok(format!(
                "<key>AssociatedBundleIdentifiers</key>\n    <array>\n        <string>{}</string>\n    </array>",
                escape_xml(bundle)
            ))
        })
        .transpose()?
        .unwrap_or_default();
    Ok(TEMPLATE
        // A comment in the template keeps the asset itself valid XML.
        .replace("<!--__GIT_SAME_ASSOCIATED_BUNDLE__-->", &associated)
        .replace("__GIT_SAME_HELPER__", &escape_path(&paths.helper)?)
        .replace("__GIT_SAME_HOME__", &escape_path(home)?)
        .replace("__GIT_SAME_STDOUT__", &escape_path(&paths.stdout_log)?)
        .replace("__GIT_SAME_STDERR__", &escape_path(&paths.stderr_log)?))
}

fn escape_path(path: &Path) -> Result<String, MonitorAgentError> {
    let value = path.to_str().ok_or_else(|| {
        MonitorAgentError::Configuration(format!(
            "LaunchAgent path is not valid UTF-8: {}",
            path.display()
        ))
    })?;
    validate_xml(value)?;
    Ok(escape_xml(value))
}

fn validate_xml(value: &str) -> Result<(), MonitorAgentError> {
    if value.chars().all(|c| {
        matches!(c as u32, 0x9 | 0xA | 0xD | 0x20..=0xD7FF | 0xE000..=0xFFFD | 0x10000..=0x10FFFF)
    }) {
        Ok(())
    } else {
        Err(MonitorAgentError::Configuration(
            "LaunchAgent value contains a character forbidden by XML 1.0".to_string(),
        ))
    }
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
