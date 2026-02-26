//! Status command handler.

use crate::cli::StatusArgs;
use crate::config::{Config, WorkspaceManager};
use crate::discovery::DiscoveryOrchestrator;
use crate::errors::Result;
use crate::git::{GitOperations, ShellGit};
use crate::output::{format_count, Output};

/// Show status of repositories.
pub async fn run(args: &StatusArgs, config: &Config, output: &Output) -> Result<()> {
    let mut workspace = WorkspaceManager::resolve(args.workspace.as_deref(), config)?;

    // Ensure base path exists (offer to fix if user moved it)
    super::ensure_base_path(&mut workspace, output)?;
    let base_path = workspace.expanded_base_path();

    let structure = workspace.structure.as_deref().unwrap_or(&config.structure);

    // Scan local repositories
    let git = ShellGit::new();
    let orchestrator = DiscoveryOrchestrator::new(workspace.filters.clone(), structure.to_string());
    let local_repos = orchestrator.scan_local(&base_path, &git);

    if local_repos.is_empty() {
        output.warn("No repositories found");
        return Ok(());
    }

    output.info(&format_count(local_repos.len(), "repositories found"));

    // Get status for each
    let mut uncommitted_count = 0;
    let mut behind_count = 0;

    for (path, org, name) in &local_repos {
        let status = git.status(path);

        match status {
            Ok(s) => {
                let is_uncommitted = s.is_uncommitted || s.has_untracked;
                let is_behind = s.behind > 0;

                // Apply filters
                if args.uncommitted && !is_uncommitted {
                    continue;
                }
                if args.behind && !is_behind {
                    continue;
                }
                if !args.org.is_empty() && !args.org.contains(org) {
                    continue;
                }

                if is_uncommitted {
                    uncommitted_count += 1;
                }
                if is_behind {
                    behind_count += 1;
                }

                // Print status
                let full_name = format!("{}/{}", org, name);
                if args.detailed {
                    println!("{}", full_name);
                    println!("  Branch: {}", s.branch);
                    if s.ahead > 0 || s.behind > 0 {
                        println!("  Ahead: {}, Behind: {}", s.ahead, s.behind);
                    }
                    if s.is_uncommitted {
                        println!("  Status: uncommitted changes");
                    }
                    if s.has_untracked {
                        println!("  Status: has untracked files");
                    }
                } else {
                    let mut indicators = Vec::new();
                    if is_uncommitted {
                        indicators.push("*".to_string());
                    }
                    if s.ahead > 0 {
                        indicators.push(format!("+{}", s.ahead));
                    }
                    if s.behind > 0 {
                        indicators.push(format!("-{}", s.behind));
                    }

                    if indicators.is_empty() {
                        println!("  {} (clean)", full_name);
                    } else {
                        println!("  {} [{}]", full_name, indicators.join(", "));
                    }
                }
            }
            Err(e) => {
                output.verbose(&format!("  {}/{} - error: {}", org, name, e));
            }
        }
    }

    // Summary
    println!();
    if uncommitted_count > 0 {
        output.warn(&format!(
            "{} repositories have uncommitted changes",
            uncommitted_count
        ));
    }
    if behind_count > 0 {
        output.info(&format!(
            "{} repositories are behind upstream",
            behind_count
        ));
    }
    if uncommitted_count == 0 && behind_count == 0 {
        output.success("All repositories are clean and up to date");
    }

    Ok(())
}

#[cfg(test)]
#[path = "status_tests.rs"]
mod tests;
