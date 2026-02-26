//! Workspace management command handler.

use crate::cli::{WorkspaceArgs, WorkspaceCommand};
use crate::config::{Config, WorkspaceManager};
use crate::errors::Result;
use crate::output::Output;

/// Run the workspace command.
pub fn run(args: &WorkspaceArgs, config: &Config, output: &Output) -> Result<()> {
    match &args.command {
        WorkspaceCommand::List => list(config, output),
        WorkspaceCommand::Default(default_args) => {
            if default_args.clear {
                clear_default(output)
            } else if let Some(ref name) = default_args.name {
                set_default(name, output)
            } else {
                show_default(config, output)
            }
        }
    }
}

fn list(config: &Config, output: &Output) -> Result<()> {
    let workspaces = WorkspaceManager::list()?;

    if workspaces.is_empty() {
        output.info("No workspaces configured. Run 'gisa setup' to create one.");
        return Ok(());
    }

    let default_name = config.default_workspace.as_deref().unwrap_or("");

    for ws in &workspaces {
        let marker = if ws.name == default_name { "*" } else { " " };
        let last_synced = ws.last_synced.as_deref().unwrap_or("never");
        let org_info = if ws.orgs.is_empty() {
            "all orgs".to_string()
        } else {
            format!("{} orgs", ws.orgs.len())
        };
        let provider_label = ws.provider.kind.display_name();

        output.plain(&format!(
            "  {} {}  ({}, {}, last synced: {})",
            marker, ws.base_path, provider_label, org_info, last_synced
        ));
    }

    if !default_name.is_empty() {
        if let Ok(default_ws) = WorkspaceManager::load(default_name) {
            output.plain("");
            output.info(&format!("Default: {}", default_ws.display_label()));
        }
    }

    Ok(())
}

fn show_default(config: &Config, output: &Output) -> Result<()> {
    match &config.default_workspace {
        Some(name) => {
            if let Ok(ws) = WorkspaceManager::load(name) {
                output.info(&format!("Default workspace: {}", ws.display_label()));
            } else {
                output.info(&format!("Default workspace: {} (not found)", name));
            }
        }
        None => output.info("No default workspace set. Use 'gisa workspace default <path>'."),
    }
    Ok(())
}

fn set_default(name_or_path: &str, output: &Output) -> Result<()> {
    // Try name first (backward compat), then path
    let ws = match WorkspaceManager::load(name_or_path) {
        Ok(ws) => ws,
        Err(_) => WorkspaceManager::load_by_path(name_or_path)?,
    };

    Config::save_default_workspace(Some(&ws.name))?;
    output.success(&format!(
        "Default workspace set to '{}'",
        ws.display_label()
    ));
    Ok(())
}

fn clear_default(output: &Output) -> Result<()> {
    Config::save_default_workspace(None)?;
    output.success("Default workspace cleared");
    Ok(())
}

#[cfg(test)]
#[path = "workspace_tests.rs"]
mod tests;
