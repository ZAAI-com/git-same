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

        println!(
            "  {} {:<16} {}  ({}, last synced: {})",
            marker, ws.name, ws.base_path, org_info, last_synced
        );
    }

    if !default_name.is_empty() {
        println!();
        output.info(&format!("Default workspace: {}", default_name));
    }

    Ok(())
}

fn show_default(config: &Config, output: &Output) -> Result<()> {
    match &config.default_workspace {
        Some(name) => output.info(&format!("Default workspace: {}", name)),
        None => output.info("No default workspace set. Use 'gisa workspace default <name>'."),
    }
    Ok(())
}

fn set_default(name: &str, output: &Output) -> Result<()> {
    // Validate workspace exists
    WorkspaceManager::load(name)?;

    Config::save_default_workspace(Some(name))?;
    output.success(&format!("Default workspace set to '{}'", name));
    Ok(())
}

fn clear_default(output: &Output) -> Result<()> {
    Config::save_default_workspace(None)?;
    output.success("Default workspace cleared");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::output::Verbosity;

    fn quiet_output() -> Output {
        Output::new(Verbosity::Quiet, false)
    }

    #[test]
    fn test_show_default_none() {
        let config = Config::default();
        let output = quiet_output();
        let result = show_default(&config, &output);
        assert!(result.is_ok());
    }

    #[test]
    fn test_show_default_some() {
        let config = Config {
            default_workspace: Some("my-ws".to_string()),
            ..Config::default()
        };
        let output = quiet_output();
        let result = show_default(&config, &output);
        assert!(result.is_ok());
    }

    #[test]
    fn test_list_empty() {
        // This test may fail if user has workspaces configured;
        // the actual CRUD tests are in workspace_manager.rs
        let config = Config::default();
        let output = quiet_output();
        // Just verify it doesn't panic
        let _ = list(&config, &output);
    }
}
