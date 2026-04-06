//! Atomic JSON status file writer and reader.
//!
//! The daemon writes the status file atomically by writing to a temporary
//! file first, then renaming it. This ensures the FinderSync extension
//! never reads a partial/corrupt file.

use crate::errors::AppError;
use crate::types::finder_status::FinderStatus;
use std::path::{Path, PathBuf};

/// Writes and reads the Finder status JSON file atomically.
#[derive(Debug, Clone)]
pub struct StatusFileWriter {
    path: PathBuf,
}

impl StatusFileWriter {
    /// Creates a writer for the given status file path.
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    /// The path this writer writes to.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Writes the status atomically (write to temp, then rename).
    pub fn write(&self, status: &FinderStatus) -> Result<(), AppError> {
        let json = serde_json::to_string_pretty(status)
            .map_err(|e| AppError::config(format!("Failed to serialize finder status: {}", e)))?;

        let temp_path = self.path.with_extension("json.tmp");

        // Ensure parent directory exists
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                AppError::path(format!(
                    "Failed to create directory '{}': {}",
                    parent.display(),
                    e
                ))
            })?;
        }

        // Write to temp file
        std::fs::write(&temp_path, &json).map_err(|e| {
            AppError::path(format!(
                "Failed to write temp status file '{}': {}",
                temp_path.display(),
                e
            ))
        })?;

        // Atomic rename
        std::fs::rename(&temp_path, &self.path).map_err(|e| {
            AppError::path(format!(
                "Failed to rename '{}' → '{}': {}",
                temp_path.display(),
                self.path.display(),
                e
            ))
        })?;

        Ok(())
    }

    /// Reads and parses the status file.
    pub fn read(&self) -> Result<FinderStatus, AppError> {
        let content = std::fs::read_to_string(&self.path).map_err(|e| {
            AppError::path(format!(
                "Failed to read status file '{}': {}",
                self.path.display(),
                e
            ))
        })?;

        serde_json::from_str(&content)
            .map_err(|e| AppError::config(format!("Failed to parse status file: {}", e)))
    }

    /// Checks if the status file exists.
    pub fn exists(&self) -> bool {
        self.path.exists()
    }
}

#[cfg(test)]
#[path = "status_file_tests.rs"]
mod tests;
