//! Reset command handler.
//!
//! Removes gisa configuration, workspace configs, and caches.
//! Supports interactive scope selection or `--force` for scripting.

use crate::cli::ResetArgs;
use chrono::{DateTime, Utc};
use git_same_core::config::{Config, WorkspaceConfig, WorkspaceManager};
use git_same_core::errors::{AppError, Result};
use git_same_core::output::Output;
use std::io::{self, BufRead, Write};
use std::path::PathBuf;

/// What scope of reset to perform.
enum ResetScope {
    Everything,
    ConfigOnly,
    AllWorkspaces,
    Workspace(PathBuf),
}

/// Rich detail about a single workspace for display.
struct WorkspaceDetail {
    root_path: PathBuf,
    orgs: Vec<String>,
    last_synced: Option<String>,
    dot_dir: PathBuf,
    cache_size: Option<u64>,
}

/// Everything that could be removed.
struct ResetTarget {
    config_dir: PathBuf,
    config_file: Option<PathBuf>,
    workspaces: Vec<WorkspaceDetail>,
}

impl ResetTarget {
    fn is_empty(&self) -> bool {
        self.config_file.is_none() && self.workspaces.is_empty()
    }

    fn has_workspaces(&self) -> bool {
        !self.workspaces.is_empty()
    }
}

/// Run the reset command.
pub async fn run(args: &ResetArgs, output: &Output) -> Result<()> {
    let target = discover_targets()?;

    if target.is_empty() {
        output.info("Nothing to reset — gisa is not configured.");
        return Ok(());
    }

    // --force: delete everything, no prompts
    if args.force {
        display_detailed_targets(&ResetScope::Everything, &target, output);
        execute_reset(&ResetScope::Everything, &target, output)?;
        nudge_daemon_refresh().await;
        return Ok(());
    }

    // Interactive: ask what to reset
    let scope = prompt_scope(&target)?;
    display_detailed_targets(&scope, &target, output);

    if !confirm("\nAre you sure? [y/N] ")? {
        output.info("Reset cancelled.");
        return Ok(());
    }

    execute_reset(&scope, &target, output)?;
    nudge_daemon_refresh().await;
    Ok(())
}

#[cfg(unix)]
async fn nudge_daemon_refresh() {
    use git_same_core::ipc::{IpcConfig, UnixSocketClient};
    let Ok(cfg) = IpcConfig::default_path() else {
        return;
    };
    let client = UnixSocketClient::new(cfg.socket_path());
    if let Err(e) = client.refresh_all().await {
        tracing::debug!(error = %e, "Monitor refresh nudge skipped");
    }
}

#[cfg(not(unix))]
async fn nudge_daemon_refresh() {}

/// Discover what files and directories exist that could be removed.
fn discover_targets() -> Result<ResetTarget> {
    let config_path = Config::default_path()?;
    let config_dir = config_path
        .parent()
        .ok_or_else(|| AppError::config("Cannot determine config directory"))?
        .to_path_buf();

    let config_file = if config_path.exists() {
        Some(config_path)
    } else {
        None
    };

    let workspaces = WorkspaceManager::list()?
        .iter()
        .map(build_workspace_detail)
        .collect::<Result<Vec<_>>>()?;

    Ok(ResetTarget {
        config_dir,
        config_file,
        workspaces,
    })
}

/// Build rich detail for a workspace.
fn build_workspace_detail(ws: &WorkspaceConfig) -> Result<WorkspaceDetail> {
    let dot_dir = WorkspaceManager::dot_dir(&ws.root_path);
    let cache_file = WorkspaceManager::cache_path(&ws.root_path);

    let cache_size = if cache_file.exists() {
        std::fs::metadata(&cache_file).map(|m| m.len()).ok()
    } else {
        None
    };

    Ok(WorkspaceDetail {
        root_path: ws.root_path.clone(),
        orgs: ws.orgs.clone(),
        last_synced: ws.last_synced.clone(),
        dot_dir,
        cache_size,
    })
}

/// Display detailed information about what will be deleted.
fn display_detailed_targets(scope: &ResetScope, target: &ResetTarget, output: &Output) {
    output.warn("The following will be permanently deleted:");

    match scope {
        ResetScope::Everything => {
            if let Some(ref path) = target.config_file {
                output.info(&format!("  Global config: {}", path.display()));
            }
            for ws in &target.workspaces {
                display_workspace_detail(ws, output);
            }
        }
        ResetScope::ConfigOnly => {
            if let Some(ref path) = target.config_file {
                output.info(&format!("  Global config: {}", path.display()));
            }
        }
        ResetScope::AllWorkspaces => {
            for ws in &target.workspaces {
                display_workspace_detail(ws, output);
            }
        }
        ResetScope::Workspace(path) => {
            if let Some(ws) = target.workspaces.iter().find(|w| w.root_path == *path) {
                display_workspace_detail(ws, output);
            }
        }
    }
}

/// Display detail for a single workspace.
fn display_workspace_detail(ws: &WorkspaceDetail, output: &Output) {
    let path_display = git_same_core::config::workspace::tilde_collapse_path(&ws.root_path);
    output.info(&format!("  Workspace at {}:", path_display));

    if ws.orgs.is_empty() {
        output.info("    Orgs:        (all)");
    } else {
        output.info(&format!(
            "    Orgs:        {} ({})",
            ws.orgs.join(", "),
            ws.orgs.len()
        ));
    }

    let synced = ws
        .last_synced
        .as_deref()
        .map(humanize_timestamp)
        .unwrap_or_else(|| "never".to_string());
    output.info(&format!("    Last synced: {}", synced));

    if let Some(size) = ws.cache_size {
        output.info(&format!("    Cache:       {}", format_bytes(size)));
    }

    output.info(&format!("    Config dir:  {}", ws.dot_dir.display()));
}

/// Execute the reset based on scope.
fn execute_reset(scope: &ResetScope, target: &ResetTarget, output: &Output) -> Result<()> {
    let mut had_errors = false;

    match scope {
        ResetScope::Everything => {
            for ws in &target.workspaces {
                had_errors |= !remove_workspace_dir(ws, output);
            }
            if let Some(ref path) = target.config_file {
                had_errors |= !remove_file(path, "config", output);
            }
            try_remove_empty_dir(&target.config_dir, output);
        }
        ResetScope::ConfigOnly => {
            if let Some(ref path) = target.config_file {
                had_errors |= !remove_file(path, "config", output);
            }
        }
        ResetScope::AllWorkspaces => {
            for ws in &target.workspaces {
                had_errors |= !remove_workspace_dir(ws, output);
            }
        }
        ResetScope::Workspace(path) => {
            if let Some(ws) = target.workspaces.iter().find(|w| w.root_path == *path) {
                had_errors |= !remove_workspace_dir(ws, output);
            } else {
                output.warn(&format!("Workspace '{}' not found.", path.display()));
                had_errors = true;
            }
        }
    }

    if had_errors {
        Err(AppError::config(
            "Reset completed with one or more removal errors.",
        ))
    } else {
        match scope {
            ResetScope::Everything => {
                output.success("Reset complete. Run 'gisa init' to start fresh.");
            }
            ResetScope::ConfigOnly => {
                output.success("Global config removed.");
            }
            ResetScope::AllWorkspaces => {
                output.success("All workspaces removed.");
            }
            ResetScope::Workspace(path) => {
                output.success(&format!("Workspace '{}' removed.", path.display()));
            }
        }
        Ok(())
    }
}

fn remove_workspace_dir(ws: &WorkspaceDetail, output: &Output) -> bool {
    let path_display = git_same_core::config::workspace::tilde_collapse_path(&ws.root_path);
    match std::fs::remove_dir_all(&ws.dot_dir) {
        Ok(()) => {
            // Also unregister from global config
            let _ = Config::remove_from_registry(&path_display);
            output.success(&format!("Removed workspace config at {}", path_display));
            true
        }
        Err(e) => {
            output.warn(&format!(
                "Failed to remove workspace config at {}: {}",
                path_display, e
            ));
            false
        }
    }
}

fn remove_file(path: &PathBuf, label: &str, output: &Output) -> bool {
    match std::fs::remove_file(path) {
        Ok(()) => {
            output.success(&format!("Removed {}: {}", label, path.display()));
            true
        }
        Err(e) => {
            output.warn(&format!("Failed to remove {}: {}", label, e));
            false
        }
    }
}

fn try_remove_empty_dir(dir: &PathBuf, output: &Output) {
    if dir.exists() {
        match std::fs::remove_dir(dir) {
            Ok(()) => output.verbose(&format!("Removed directory: {}", dir.display())),
            Err(_) => output.verbose(&format!(
                "Config directory not empty, leaving: {}",
                dir.display()
            )),
        }
    }
}

// --- Interactive prompts (all write to stderr) ---

/// Prompt user to select what to reset.
fn prompt_scope(target: &ResetTarget) -> Result<ResetScope> {
    eprintln!("What would you like to reset?");

    let mut options: Vec<(&str, ResetScope)> = Vec::new();

    if target.config_file.is_some() && target.has_workspaces() {
        options.push((
            "Everything (global config + all workspaces)",
            ResetScope::Everything,
        ));
    }

    if target.config_file.is_some() {
        options.push(("Global config only", ResetScope::ConfigOnly));
    }

    if target.workspaces.len() > 1 {
        options.push(("All workspaces", ResetScope::AllWorkspaces));
    }

    if target.has_workspaces() {
        options.push((
            "A specific workspace",
            ResetScope::Workspace(PathBuf::new()),
        ));
    }

    // If only one option, skip the menu
    if options.len() == 1 {
        let (_, scope) = options.remove(0);
        return match scope {
            ResetScope::Workspace(_) => prompt_workspace(&target.workspaces),
            other => Ok(other),
        };
    }

    for (i, (label, _)) in options.iter().enumerate() {
        eprintln!("  {}. {}", i + 1, label);
    }

    let choice = prompt_number("> ", options.len())?;
    let (_, scope) = options.remove(choice - 1);

    match scope {
        ResetScope::Workspace(_) => prompt_workspace(&target.workspaces),
        other => Ok(other),
    }
}

/// Prompt user to pick a specific workspace.
fn prompt_workspace(workspaces: &[WorkspaceDetail]) -> Result<ResetScope> {
    eprintln!("\nSelect a workspace to delete:");
    for (i, ws) in workspaces.iter().enumerate() {
        let path_display = git_same_core::config::workspace::tilde_collapse_path(&ws.root_path);
        let orgs = if ws.orgs.is_empty() {
            "all orgs".to_string()
        } else {
            format!("{} org(s)", ws.orgs.len())
        };
        let synced = ws
            .last_synced
            .as_deref()
            .map(humanize_timestamp)
            .unwrap_or_else(|| "never synced".to_string());
        eprintln!("  {}. {}  ({}, {})", i + 1, path_display, orgs, synced);
    }

    let choice = prompt_number("> ", workspaces.len())?;
    Ok(ResetScope::Workspace(
        workspaces[choice - 1].root_path.clone(),
    ))
}

/// Read a number from stdin (1-based, within max).
fn prompt_number(prompt: &str, max: usize) -> Result<usize> {
    loop {
        eprint!("{}", prompt);
        io::stderr().flush()?;

        let stdin = io::stdin();
        let mut line = String::new();
        let bytes_read = stdin.lock().read_line(&mut line)?;
        if bytes_read == 0 {
            return Err(AppError::Interrupted);
        }

        match line.trim().parse::<usize>() {
            Ok(n) if n >= 1 && n <= max => return Ok(n),
            _ => eprintln!("Please enter a number between 1 and {}.", max),
        }
    }
}

/// Prompt the user for y/N confirmation.
fn confirm(prompt: &str) -> Result<bool> {
    eprint!("{}", prompt);
    io::stderr().flush()?;

    let stdin = io::stdin();
    let mut line = String::new();
    stdin.lock().read_line(&mut line)?;

    let answer = line.trim().to_lowercase();
    Ok(answer == "y" || answer == "yes")
}

// --- Formatting helpers ---

/// Humanize an ISO 8601 timestamp to a relative string like "2h ago".
fn humanize_timestamp(ts: &str) -> String {
    let parsed = ts
        .parse::<DateTime<Utc>>()
        .or_else(|_| DateTime::parse_from_rfc3339(ts).map(|dt| dt.with_timezone(&Utc)));

    let Ok(dt) = parsed else {
        return ts.to_string();
    };

    let duration = Utc::now().signed_duration_since(dt);

    if duration.num_days() > 30 {
        format!("{}mo ago", duration.num_days() / 30)
    } else if duration.num_days() > 0 {
        format!("{}d ago", duration.num_days())
    } else if duration.num_hours() > 0 {
        format!("{}h ago", duration.num_hours())
    } else if duration.num_minutes() > 0 {
        format!("{}m ago", duration.num_minutes())
    } else {
        "just now".to_string()
    }
}

/// Format bytes to human-readable string.
fn format_bytes(bytes: u64) -> String {
    if bytes >= 1_048_576 {
        format!("{:.1} MB", bytes as f64 / 1_048_576.0)
    } else if bytes >= 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{} B", bytes)
    }
}

#[cfg(test)]
#[path = "reset_tests.rs"]
mod tests;
