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
/// A machine with no configuration keeps none: `gisa monitor --stop` must not
/// materialise a full `config.toml` the user never asked for, which would also
/// turn "No configuration found. Run 'gisa init'." into an empty workspace
/// list. The durable half of a Stop is `launchctl disable`, which
/// `monitoring_enabled` honours on its own and which `gisa reset` already
/// relies on outliving the configuration file.
pub fn set_monitor_autostart(path: &Path, autostart: bool) -> Result<(), AppError> {
    if !path.exists() {
        return Ok(());
    }
    edit_document(path, |doc| {
        table_mut(doc, "monitor")?["autostart"] = value(autostart);
        Ok(())
    })
}

/// Top-level keys the settings form owns.
const SETTINGS_KEYS: &[&str] = &[
    "structure",
    "concurrency",
    "sync_mode",
    "default_workspace",
    "refresh_interval",
    "workspaces",
];
/// Tables the settings form owns entirely.
const SETTINGS_TABLES: &[&str] = &["clone", "filters", "finder"];

/// Saves the values of a settings form without rewriting the file.
///
/// Only keys the form models are touched. Everything else keeps its persisted
/// value, in particular `monitor.autostart` (so a stale form can never undo
/// `gisa monitor --stop`) and the whole `[ui]` section, along with comments
/// and unknown keys.
pub fn merge_settings(path: &Path, settings: &Config) -> Result<(), AppError> {
    let rendered = toml::to_string(settings)
        .map_err(|e| AppError::config(format!("Failed to serialize settings: {e}")))?
        .parse::<DocumentMut>()
        .map_err(|e| AppError::config(format!("Failed to prepare settings: {e}")))?;

    edit_document(path, |doc| {
        for key in SETTINGS_KEYS {
            match rendered.get(key) {
                Some(item) => set_if_changed(doc.as_table_mut(), key, item),
                // `None` options are not serialized: the key was cleared.
                None => {
                    doc.remove(key);
                }
            }
        }
        for name in SETTINGS_TABLES {
            if let Some(source) = rendered.get(name).and_then(Item::as_table) {
                let target = table_mut(doc, name)?;
                for (key, item) in source.iter() {
                    set_if_changed(target, key, item);
                }
            }
        }
        if let Some(interval) = rendered
            .get("monitor")
            .and_then(|monitor| monitor.get("fullscan_interval_secs"))
        {
            set_if_changed(
                table_mut(doc, "monitor")?,
                "fullscan_interval_secs",
                interval,
            );
        }
        Ok(())
    })
}

/// Leaves an unchanged value alone, and replaces a changed one in place so
/// the comments attached to its key survive.
fn set_if_changed(table: &mut Table, key: &str, item: &Item) {
    let render = |item: &Item| item.to_string().trim().to_string();
    if table.get(key).map(render) == Some(render(item)) {
        return;
    }
    match (
        table.get_mut(key).and_then(Item::as_value_mut),
        item.as_value(),
    ) {
        (Some(existing), Some(new)) => {
            let decor = existing.decor().clone();
            *existing = new.clone();
            *existing.decor_mut() = decor;
        }
        _ => {
            table.insert(key, item.clone());
        }
    }
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
