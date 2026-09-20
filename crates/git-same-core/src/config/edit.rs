//! Targeted, comment-preserving edits of the global `config.toml`.
//!
//! These writers use `toml_edit` so they change only the keys they own and
//! keep comments and unrelated values intact. They never replace a malformed
//! file: a parse failure is returned to the caller and the file is untouched.

use super::Config;
use crate::errors::AppError;
use crate::fsutil::atomic_write;
use std::path::Path;
use toml_edit::{value, DocumentMut, Item, Table};

/// Reads `monitor.autostart`. A missing file or key means `true`.
///
/// A malformed file is an error; callers must not treat it as a preference.
pub fn read_monitor_autostart(path: &Path) -> Result<bool, AppError> {
    if !path.exists() {
        return Ok(true);
    }
    let doc = load_document(path)?;
    Ok(doc
        .get("monitor")
        .and_then(|monitor| monitor.get("autostart"))
        .and_then(Item::as_bool)
        .unwrap_or(true))
}

/// Persists `monitor.autostart`, changing nothing else in the file.
///
/// Creates the default configuration only when the file is missing.
pub fn set_monitor_autostart(path: &Path, autostart: bool) -> Result<(), AppError> {
    edit_document(path, |doc| {
        table_mut(doc, "monitor")?["autostart"] = value(autostart);
        Ok(())
    })
}

/// Applies `edit` to the parsed document and writes it back atomically.
///
/// Shared by every targeted writer so they all get the same guarantees:
/// create-only-when-missing, refuse malformed input, atomic replacement.
pub fn edit_document(
    path: &Path,
    edit: impl FnOnce(&mut DocumentMut) -> Result<(), AppError>,
) -> Result<(), AppError> {
    if !path.exists() {
        atomic_write(path, Config::default_toml().as_bytes(), None).map_err(|e| {
            AppError::config(format!(
                "Failed to create default config '{}': {e}",
                path.display()
            ))
        })?;
    }
    let mut doc = load_document(path)?;
    edit(&mut doc)?;
    atomic_write(path, doc.to_string().as_bytes(), None)
        .map_err(|e| AppError::config(format!("Failed to write config '{}': {e}", path.display())))
}

/// Returns the named top-level table, creating it when absent.
pub fn table_mut<'a>(doc: &'a mut DocumentMut, name: &str) -> Result<&'a mut Table, AppError> {
    let item = doc.entry(name).or_insert_with(|| Item::Table(Table::new()));
    item.as_table_mut()
        .ok_or_else(|| AppError::config(format!("Config key '{name}' is not a table")))
}

fn load_document(path: &Path) -> Result<DocumentMut, AppError> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| AppError::config(format!("Failed to read config file: {e}")))?;
    content.parse::<DocumentMut>().map_err(|e| {
        AppError::config(format!(
            "Config file '{}' is malformed and was left untouched: {e}",
            path.display()
        ))
    })
}

#[cfg(test)]
#[path = "edit_tests.rs"]
mod tests;
