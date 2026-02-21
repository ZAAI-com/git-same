//! Status command handler.

use super::expand_path;
use crate::adapters::config::Config;
use crate::adapters::git::{GitOperations, ShellGit};
use crate::adapters::output::{format_count, Output};
use crate::cli::StatusArgs;
use crate::discovery::DiscoveryOrchestrator;
use crate::errors::{AppError, Result};

/// Show status of repositories.
pub async fn run(args: &StatusArgs, config: &Config, output: &Output) -> Result<()> {
    let base_path = expand_path(&args.base_path);
    if !base_path.exists() {
        return Err(AppError::config(format!(
            "Base path does not exist: {}",
            base_path.display()
        )));
    }

    // Scan local repositories
    let git = ShellGit::new();
    let orchestrator = DiscoveryOrchestrator::new(config.filters.clone(), config.structure.clone());
    let local_repos = orchestrator.scan_local(&base_path, &git);

    if local_repos.is_empty() {
        output.warn("No repositories found");
        return Ok(());
    }

    output.info(&format_count(local_repos.len(), "repositories found"));

    // Get status for each
    let mut dirty_count = 0;
    let mut behind_count = 0;

    for (path, org, name) in &local_repos {
        let status = git.status(path);

        match status {
            Ok(s) => {
                let is_dirty = s.is_dirty || s.has_untracked;
                let is_behind = s.behind > 0;

                if is_dirty {
                    dirty_count += 1;
                }
                if is_behind {
                    behind_count += 1;
                }

                // Apply filters
                if args.dirty && !is_dirty {
                    continue;
                }
                if args.behind && !is_behind {
                    continue;
                }
                if !args.org.is_empty() && !args.org.contains(org) {
                    continue;
                }

                // Print status
                let full_name = format!("{}/{}", org, name);
                if args.detailed {
                    println!("{}", full_name);
                    println!("  Branch: {}", s.branch);
                    if s.ahead > 0 || s.behind > 0 {
                        println!("  Ahead: {}, Behind: {}", s.ahead, s.behind);
                    }
                    if s.is_dirty {
                        println!("  Status: dirty (uncommitted changes)");
                    }
                    if s.has_untracked {
                        println!("  Status: has untracked files");
                    }
                } else {
                    let mut indicators = Vec::new();
                    if is_dirty {
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
    if dirty_count > 0 {
        output.warn(&format!(
            "{} repositories have uncommitted changes",
            dirty_count
        ));
    }
    if behind_count > 0 {
        output.info(&format!(
            "{} repositories are behind upstream",
            behind_count
        ));
    }
    if dirty_count == 0 && behind_count == 0 {
        output.success("All repositories are clean and up to date");
    }

    Ok(())
}
