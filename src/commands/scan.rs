//! Scan command — find unregistered .git-same/ workspace folders.

use crate::cli::ScanArgs;
use crate::config::{Config, WorkspaceStore};
use crate::errors::{AppError, Result};
use crate::output::Output;
use std::path::{Path, PathBuf};

/// Run the scan command.
pub fn run(args: &ScanArgs, config_path: Option<&Path>, output: &Output) -> Result<()> {
    let root = match &args.path {
        Some(p) => p.clone(),
        None => std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
    };

    let root = std::fs::canonicalize(&root).unwrap_or(root);
    output.info(&format!(
        "Scanning {} (depth {})",
        root.display(),
        args.depth
    ));

    let found = scan_for_workspaces(&root, args.depth);

    if found.is_empty() {
        output.info("No .git-same/ workspaces found.");
        return Ok(());
    }

    // Load existing registry to flag already-registered workspaces
    let global = match config_path {
        Some(path) => Config::load_from(path),
        None => Config::load(),
    }?;
    let registered: std::collections::HashSet<PathBuf> = global
        .workspaces
        .iter()
        .map(|p| {
            let expanded = shellexpand::tilde(p);
            std::fs::canonicalize(expanded.as_ref())
                .unwrap_or_else(|_| PathBuf::from(expanded.as_ref()))
        })
        .collect();

    let mut unregistered_count = 0usize;
    let mut register_failures = Vec::new();
    for ws_root in &found {
        let is_registered = registered.contains(ws_root);
        let tilde = crate::config::workspace::tilde_collapse_path(ws_root);
        if is_registered {
            output.plain(&format!("  [registered]   {}", tilde));
        } else {
            output.plain(&format!("  [unregistered] {}", tilde));
            unregistered_count += 1;

            if args.register {
                match WorkspaceStore::load(ws_root) {
                    Ok(ws) => {
                        let save_result = match config_path {
                            Some(path) => WorkspaceStore::save_with_registry_config_path(&ws, path),
                            None => WorkspaceStore::save(&ws),
                        };
                        match save_result {
                            Ok(()) => {
                                output.success(&format!("    Registered: {}", tilde));
                                unregistered_count = unregistered_count.saturating_sub(1);
                            }
                            Err(e) => {
                                output.warn(&format!("    Failed to register {}: {}", tilde, e));
                                register_failures.push(format!("{}: {}", tilde, e));
                            }
                        }
                    }
                    Err(e) => {
                        output.warn(&format!("    Skipping {}: {}", tilde, e));
                        register_failures.push(format!("{}: {}", tilde, e));
                    }
                }
            }
        }
    }

    output.plain("");
    output.info(&format!(
        "Found {} workspace(s): {} registered, {} unregistered{}",
        found.len(),
        found.len() - unregistered_count,
        unregistered_count,
        if unregistered_count > 0 && !args.register {
            " (use --register to add them)"
        } else {
            ""
        }
    ));

    if !register_failures.is_empty() {
        let first = register_failures
            .first()
            .map(String::as_str)
            .unwrap_or("unknown error");
        return Err(AppError::config(format!(
            "Failed to register {} workspace(s). First error: {}",
            register_failures.len(),
            first
        )));
    }

    Ok(())
}

/// Recursively scan for directories containing `.git-same/config.toml`.
fn scan_for_workspaces(root: &Path, max_depth: usize) -> Vec<PathBuf> {
    let mut results = Vec::new();
    scan_recursive(root, 0, max_depth, &mut results);
    results.sort();
    results
}

fn scan_recursive(dir: &Path, depth: usize, max_depth: usize, results: &mut Vec<PathBuf>) {
    if depth > max_depth {
        return;
    }

    // Check if this directory is a workspace root
    let config_path = WorkspaceStore::config_path(dir);
    if config_path.exists() {
        if let Ok(canonical) = std::fs::canonicalize(dir) {
            results.push(canonical);
        }
        // Don't recurse into workspace directories
        return;
    }

    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        // Skip hidden dirs (except .git-same itself is already handled above)
        if name.starts_with('.') {
            continue;
        }
        scan_recursive(&path, depth + 1, max_depth, results);
    }
}

#[cfg(test)]
#[path = "scan_tests.rs"]
mod tests;
