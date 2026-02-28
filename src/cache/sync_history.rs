use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::debug;

use crate::tui::app::SyncHistoryEntry;

const HISTORY_VERSION: u32 = 1;
const MAX_HISTORY_ENTRIES: usize = 50;

#[derive(Debug, Serialize, Deserialize)]
struct SyncHistoryFile {
    version: u32,
    entries: Vec<SyncHistoryEntry>,
}

/// Manages per-workspace sync history persistence.
///
/// History is stored at `<workspace-root>/.git-same/sync-history.json`.
pub struct SyncHistoryManager {
    path: PathBuf,
}

impl SyncHistoryManager {
    /// Create a history manager for a specific workspace root path.
    pub fn for_workspace(root: &Path) -> Result<Self> {
        let path = crate::config::WorkspaceStore::sync_history_path(root);
        Ok(Self { path })
    }

    /// Load sync history from disk. Returns empty vec if file doesn't exist.
    pub fn load(&self) -> Result<Vec<SyncHistoryEntry>> {
        if !self.path.exists() {
            return Ok(Vec::new());
        }
        let content = fs::read_to_string(&self.path).context("Failed to read sync history file")?;
        let file: SyncHistoryFile =
            serde_json::from_str(&content).context("Failed to parse sync history")?;
        if file.version != HISTORY_VERSION {
            debug!(
                file_version = file.version,
                current_version = HISTORY_VERSION,
                "Sync history version mismatch, starting fresh"
            );
            return Ok(Vec::new());
        }
        Ok(file.entries)
    }

    /// Save sync history to disk, keeping only the most recent entries.
    pub fn save(&self, entries: &[SyncHistoryEntry]) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).context("Failed to create history directory")?;
        }
        let capped: Vec<SyncHistoryEntry> = entries
            .iter()
            .rev()
            .take(MAX_HISTORY_ENTRIES)
            .rev()
            .cloned()
            .collect();
        let file = SyncHistoryFile {
            version: HISTORY_VERSION,
            entries: capped,
        };
        let json =
            serde_json::to_string_pretty(&file).context("Failed to serialize sync history")?;
        fs::write(&self.path, &json).context("Failed to write sync history")?;
        debug!(
            path = %self.path.display(),
            entries = file.entries.len(),
            "Saved sync history"
        );
        Ok(())
    }
}

#[cfg(test)]
#[path = "sync_history_tests.rs"]
mod tests;
