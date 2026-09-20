//! Persistent installation record (`install.json`).

use crate::errors::MonitorAgentError;
use crate::fsutil::atomic_write;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Who installed the managed helper, and therefore who may replace it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OwnerKind {
    /// Installed by the Homebrew cask. Only the cask installer writes this.
    HomebrewCask,
    /// Installed from a `Git-Same.app` bundle.
    App,
    /// Installed from a standalone CLI (formula, cargo, download).
    Cli,
}

impl OwnerKind {
    /// App bundles, whether or not Homebrew placed them.
    pub fn is_app(self) -> bool {
        matches!(self, OwnerKind::HomebrewCask | OwnerKind::App)
    }
}

/// Versioned record of the installed helper.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstallRecord {
    pub schema_version: u32,
    pub owner_kind: OwnerKind,
    /// Stable app bundle or CLI installation path.
    pub owner_path: PathBuf,
    /// Stable executable used for future updates.
    pub source_binary: PathBuf,
    pub binary_version: String,
    /// Identity of the installed helper.
    pub binary_sha256: String,
    pub installed_at: String,
    /// Size and mtime of the source when it was installed: a cheap change
    /// check so a healthy ensure does not hash the executable every time.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_stamp: Option<(u64, u64)>,
}

impl InstallRecord {
    pub const SCHEMA_VERSION: u32 = 1;

    /// Loads the record. `Ok(None)` when absent.
    ///
    /// An unreadable or newer-schema record is an error: ownership that
    /// cannot be interpreted must never be silently overwritten.
    pub fn load(path: &Path) -> Result<Option<Self>, MonitorAgentError> {
        let content = match std::fs::read(path) {
            Ok(content) => content,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(MonitorAgentError::io("Failed to read install record", e)),
        };
        let unreadable = |reason: String| {
            MonitorAgentError::Configuration(format!(
                "install record '{}' {reason}. Run 'gisa monitor --uninstall' with the newest \
                 installed Git-Same, then 'gisa monitor --start'",
                path.display()
            ))
        };
        let value: serde_json::Value = serde_json::from_slice(&content)
            .map_err(|e| unreadable(format!("is not valid JSON ({e})")))?;
        let version = value.get("schema_version").and_then(|v| v.as_u64());
        if version != Some(u64::from(Self::SCHEMA_VERSION)) {
            return Err(unreadable(format!(
                "has unsupported schema version {version:?}"
            )));
        }
        serde_json::from_value(value)
            .map(Some)
            .map_err(|e| unreadable(format!("could not be interpreted ({e})")))
    }

    pub fn save(&self, path: &Path) -> Result<(), MonitorAgentError> {
        let json = serde_json::to_vec_pretty(self)
            .map_err(|e| MonitorAgentError::io("Failed to encode install record", e.into()))?;
        atomic_write(path, &json, Some(0o600))
            .map_err(|e| MonitorAgentError::io("Failed to write install record", e))
    }
}

/// `(len, mtime nanoseconds)` of a file, for cheap change detection. Records
/// written by older builds used seconds in the second slot; they simply cause
/// one safe hash comparison and refresh after upgrade.
pub fn file_stamp(path: &Path) -> Option<(u64, u64)> {
    let metadata = std::fs::metadata(path).ok()?;
    let modified = metadata
        .modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?;
    Some((metadata.len(), u64::try_from(modified.as_nanos()).ok()?))
}

/// SHA-256 of a file as lowercase hex.
pub fn sha256_file(path: &Path) -> std::io::Result<String> {
    use sha2::{Digest, Sha256};
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    std::io::copy(&mut file, &mut hasher)?;
    Ok(hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

#[cfg(test)]
#[path = "record_tests.rs"]
mod tests;
