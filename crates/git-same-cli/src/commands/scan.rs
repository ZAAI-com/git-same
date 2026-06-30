//! Scan command — find unregistered .git-same/ workspace folders.

use crate::cli::ScanArgs;
use git_same_core::config::{Config, WorkspaceStore};
use git_same_core::errors::{AppError, Result};
use git_same_core::output::Output;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Run the scan command.
pub fn run(args: &ScanArgs, config_path: Option<&Path>, output: &Output) -> Result<()> {
    let root = match &args.path {
        Some(p) => p.clone(),
        None => std::env::current_dir()
            .map_err(|e| AppError::config(format!("Failed to resolve current directory: {}", e)))?,
    };

    let root = std::fs::canonicalize(&root).map_err(|e| {
        AppError::config(format!(
            "Failed to access scan root {}: {}",
            root.display(),
            e
        ))
    })?;
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
        let tilde = git_same_core::config::workspace::tilde_collapse_path(ws_root);
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
    let mut visited = HashSet::new();
    scan_recursive(root, 0, max_depth, &mut results, &mut visited);
    results.sort();
    results.dedup();
    results
}

fn scan_recursive(
    dir: &Path,
    depth: usize,
    max_depth: usize,
    results: &mut Vec<PathBuf>,
    visited: &mut HashSet<PathBuf>,
) {
    if depth > max_depth {
        return;
    }

    let canonical_dir = std::fs::canonicalize(dir).unwrap_or_else(|_| dir.to_path_buf());
    if !visited.insert(canonical_dir.clone()) {
        return;
    }

    // Check if this directory is a workspace root
    let config_path = WorkspaceStore::config_path(&canonical_dir);
    if config_path.exists() {
        results.push(canonical_dir);
        // Don't recurse into workspace directories
        return;
    }

    let Ok(entries) = std::fs::read_dir(&canonical_dir) else {
        return;
    };

    for entry in entries.flatten() {
        // Avoid traversing symlinks to directories.
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if !file_type.is_dir() {
            continue;
        }

        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        // Skip hidden dirs (except .git-same itself is already handled above)
        if name.starts_with('.') {
            continue;
        }
        scan_recursive(&path, depth + 1, max_depth, results, visited);
    }
}

#[cfg(test)]
#[path = "scan_tests.rs"]
mod tests;
