//! Workspace management command handler.

use crate::cli::{WorkspaceArgs, WorkspaceCommand};
use git_same_core::config::{Config, WorkspaceManager};
use git_same_core::errors::Result;
use git_same_core::output::Output;

/// Run the workspace command.
pub fn run(args: &WorkspaceArgs, config: &Config, output: &Output) -> Result<()> {
    match &args.command {
        WorkspaceCommand::List => list(config, output),
        WorkspaceCommand::Default(default_args) => {
            if default_args.clear {
                clear_default(output)
            } else if let Some(ref selector) = default_args.name {
                set_default(selector, config, output)
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

    let default_path = config.default_workspace.as_deref().unwrap_or("");

    for ws in &workspaces {
        let ws_path = git_same_core::config::workspace::tilde_collapse_path(&ws.root_path);
        let marker = if ws_path == default_path { "*" } else { " " };
        let last_synced = ws.last_synced.as_deref().unwrap_or("never");
        let org_info = if ws.orgs.is_empty() {
            "all orgs".to_string()
        } else {
            format!("{} orgs", ws.orgs.len())
        };
        let provider_label = ws.provider.kind.display_name();

        output.plain(&format!(
            "  {} {}  ({}, {}, last synced: {})",
            marker, ws_path, provider_label, org_info, last_synced
        ));
    }

    if !default_path.is_empty() {
        let expanded = shellexpand::tilde(default_path);
        let root = std::path::Path::new(expanded.as_ref());
        if let Ok(default_ws) = WorkspaceManager::load(root) {
            output.plain("");
            output.info(&format!("Default: {}", default_ws.display_label()));
        }
    }

    Ok(())
}

fn show_default(config: &Config, output: &Output) -> Result<()> {
    match &config.default_workspace {
        Some(path_str) => {
            let expanded = shellexpand::tilde(path_str);
            let root = std::path::Path::new(expanded.as_ref());
            if let Ok(ws) = WorkspaceManager::load(root) {
                output.info(&format!("Default workspace: {}", ws.display_label()));
            } else {
                output.info(&format!("Default workspace: {} (not found)", path_str));
            }
        }
        None => output.info("No default workspace set. Use 'gisa workspace default <path|name>'."),
    }
    Ok(())
}

fn set_default(selector: &str, config: &Config, output: &Output) -> Result<()> {
    let ws = WorkspaceManager::resolve(Some(selector), config)?;

    let tilde_path = git_same_core::config::workspace::tilde_collapse_path(&ws.root_path);
    Config::save_default_workspace(Some(&tilde_path))?;
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
