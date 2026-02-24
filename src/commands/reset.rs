//! Reset command handler.
//!
//! Removes gisa configuration, workspace configs, and caches.
//! Supports interactive scope selection or `--force` for scripting.

use crate::cli::ResetArgs;
use crate::config::{Config, WorkspaceConfig, WorkspaceManager};
use crate::errors::{AppError, Result};
use crate::output::Output;
use chrono::{DateTime, Utc};
use std::io::{self, BufRead, Write};
use std::path::PathBuf;

/// What scope of reset to perform.
enum ResetScope {
    Everything,
    ConfigOnly,
    AllWorkspaces,
    Workspace(String),
}

/// Rich detail about a single workspace for display.
struct WorkspaceDetail {
    name: String,
    base_path: String,
    orgs: Vec<String>,
    last_synced: Option<String>,
    dir: PathBuf,
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
    Ok(())
}

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

    let workspaces = WorkspaceManager::list()
        .unwrap_or_default()
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
    let dir = WorkspaceManager::workspace_dir(&ws.name)?;
    let cache_file = WorkspaceManager::cache_path(&ws.name)?;

    let cache_size = if cache_file.exists() {
        std::fs::metadata(&cache_file).map(|m| m.len()).ok()
    } else {
        None
    };

    Ok(WorkspaceDetail {
        name: ws.name.clone(),
        base_path: ws.base_path.clone(),
        orgs: ws.orgs.clone(),
        last_synced: ws.last_synced.clone(),
        dir,
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
        ResetScope::Workspace(name) => {
            if let Some(ws) = target.workspaces.iter().find(|w| w.name == *name) {
                display_workspace_detail(ws, output);
            }
        }
    }
}

/// Display detail for a single workspace.
fn display_workspace_detail(ws: &WorkspaceDetail, output: &Output) {
    output.info(&format!("  Workspace at {}:", ws.base_path));

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

    output.info(&format!("    Directory:   {}", ws.dir.display()));
}

/// Execute the reset based on scope.
fn execute_reset(scope: &ResetScope, target: &ResetTarget, output: &Output) -> Result<()> {
    match scope {
        ResetScope::Everything => {
            for ws in &target.workspaces {
                remove_workspace_dir(ws, output);
            }
            if let Some(ref path) = target.config_file {
                remove_file(path, "config", output);
            }
            try_remove_empty_dir(&target.config_dir, output);
            output.success("Reset complete. Run 'gisa init' to start fresh.");
        }
        ResetScope::ConfigOnly => {
            if let Some(ref path) = target.config_file {
                remove_file(path, "config", output);
            }
            output.success("Global config removed.");
        }
        ResetScope::AllWorkspaces => {
            for ws in &target.workspaces {
                remove_workspace_dir(ws, output);
            }
            output.success("All workspaces removed.");
        }
        ResetScope::Workspace(name) => {
            if let Some(ws) = target.workspaces.iter().find(|w| w.name == *name) {
                remove_workspace_dir(ws, output);
                output.success(&format!("Workspace at {} removed.", ws.base_path));
            } else {
                output.warn(&format!("Workspace '{}' not found.", name));
            }
        }
    }
    Ok(())
}

fn remove_workspace_dir(ws: &WorkspaceDetail, output: &Output) {
    match std::fs::remove_dir_all(&ws.dir) {
        Ok(()) => output.success(&format!("Removed workspace at {}", ws.base_path)),
        Err(e) => output.warn(&format!(
            "Failed to remove workspace at {}: {}",
            ws.base_path, e
        )),
    }
}

fn remove_file(path: &PathBuf, label: &str, output: &Output) {
    match std::fs::remove_file(path) {
        Ok(()) => output.success(&format!("Removed {}: {}", label, path.display())),
        Err(e) => output.warn(&format!("Failed to remove {}: {}", label, e)),
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
        options.push(("A specific workspace", ResetScope::Workspace(String::new())));
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
        eprintln!("  {}. {}  ({}, {})", i + 1, ws.base_path, orgs, synced);
    }

    let choice = prompt_number("> ", workspaces.len())?;
    Ok(ResetScope::Workspace(workspaces[choice - 1].name.clone()))
}

/// Read a number from stdin (1-based, within max).
fn prompt_number(prompt: &str, max: usize) -> Result<usize> {
    loop {
        eprint!("{}", prompt);
        io::stderr().flush()?;

        let stdin = io::stdin();
        let mut line = String::new();
        stdin.lock().read_line(&mut line)?;

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
mod tests {
    use super::*;

    #[test]
    fn test_reset_target_is_empty_when_nothing_exists() {
        let target = ResetTarget {
            config_dir: PathBuf::from("/nonexistent"),
            config_file: None,
            workspaces: Vec::new(),
        };
        assert!(target.is_empty());
    }

    #[test]
    fn test_reset_target_not_empty_with_config() {
        let target = ResetTarget {
            config_dir: PathBuf::from("/some/dir"),
            config_file: Some(PathBuf::from("/some/dir/config.toml")),
            workspaces: Vec::new(),
        };
        assert!(!target.is_empty());
    }

    #[test]
    fn test_reset_target_not_empty_with_workspaces() {
        let target = ResetTarget {
            config_dir: PathBuf::from("/some/dir"),
            config_file: None,
            workspaces: vec![WorkspaceDetail {
                name: "ws1".to_string(),
                base_path: "~/github".to_string(),
                orgs: vec!["org1".to_string()],
                last_synced: None,
                dir: PathBuf::from("/some/dir/ws1"),
                cache_size: None,
            }],
        };
        assert!(!target.is_empty());
    }

    #[test]
    fn test_humanize_timestamp_hours() {
        let ts = (Utc::now() - chrono::Duration::hours(3)).to_rfc3339();
        assert_eq!(humanize_timestamp(&ts), "3h ago");
    }

    #[test]
    fn test_humanize_timestamp_days() {
        let ts = (Utc::now() - chrono::Duration::days(5)).to_rfc3339();
        assert_eq!(humanize_timestamp(&ts), "5d ago");
    }

    #[test]
    fn test_humanize_timestamp_invalid() {
        assert_eq!(humanize_timestamp("not-a-date"), "not-a-date");
    }

    #[test]
    fn test_humanize_timestamp_just_now() {
        let ts = Utc::now().to_rfc3339();
        assert_eq!(humanize_timestamp(&ts), "just now");
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(500), "500 B");
        assert_eq!(format_bytes(1024), "1.0 KB");
        assert_eq!(format_bytes(15360), "15.0 KB");
        assert_eq!(format_bytes(1_048_576), "1.0 MB");
    }

    #[test]
    fn test_display_workspace_detail_no_panic() {
        let ws = WorkspaceDetail {
            name: "test".to_string(),
            base_path: "~/github".to_string(),
            orgs: vec!["org1".to_string(), "org2".to_string()],
            last_synced: Some("2026-02-24T10:00:00Z".to_string()),
            dir: PathBuf::from("/tmp/test"),
            cache_size: Some(12345),
        };
        let output = Output::new(crate::output::Verbosity::Quiet, false);
        display_workspace_detail(&ws, &output);
    }

    #[test]
    fn test_display_detailed_targets_everything() {
        let target = ResetTarget {
            config_dir: PathBuf::from("/tmp/test"),
            config_file: Some(PathBuf::from("/tmp/test/config.toml")),
            workspaces: vec![WorkspaceDetail {
                name: "ws1".to_string(),
                base_path: "~/github".to_string(),
                orgs: Vec::new(),
                last_synced: None,
                dir: PathBuf::from("/tmp/test/ws1"),
                cache_size: None,
            }],
        };
        let output = Output::new(crate::output::Verbosity::Quiet, false);
        display_detailed_targets(&ResetScope::Everything, &target, &output);
    }

    #[test]
    fn test_display_detailed_targets_config_only() {
        let target = ResetTarget {
            config_dir: PathBuf::from("/tmp/test"),
            config_file: Some(PathBuf::from("/tmp/test/config.toml")),
            workspaces: Vec::new(),
        };
        let output = Output::new(crate::output::Verbosity::Quiet, false);
        display_detailed_targets(&ResetScope::ConfigOnly, &target, &output);
    }
}
