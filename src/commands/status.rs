//! Status command handler.

use crate::api::RepoScanService;
use crate::cli::StatusArgs;
use crate::config::{Config, WorkspaceManager};
use crate::errors::Result;
use crate::git::ShellGit;
use crate::output::{format_count, Output};

/// Show status of repositories.
pub async fn run(args: &StatusArgs, config: &Config, output: &Output) -> Result<()> {
    let workspace = WorkspaceManager::resolve(args.workspace.as_deref(), config)?;

    // Ensure base path exists (offer to fix if user moved it)
    super::ensure_base_path(&workspace, output)?;

    // Use the scan service to get full FinderRepoStatus for every repo
    let git = ShellGit::new();
    let service = RepoScanService::new(&git, config);
    let repos = service.scan_workspace(&workspace)?;

    if repos.is_empty() {
        output.warn("No repositories found");
        return Ok(());
    }

    output.info(&format_count(repos.len(), "repositories found"));

    // Get status for each
    let mut uncommitted_count = 0;
    let mut behind_count = 0;

    for repo in &repos {
        let is_uncommitted =
            repo.staged_count > 0 || repo.unstaged_count > 0 || repo.untracked_count > 0;
        let is_behind = repo.behind > 0;

        // Apply filters
        if args.uncommitted && !is_uncommitted {
            continue;
        }
        if args.behind && !is_behind {
            continue;
        }
        if !args.org.is_empty() {
            let matches_org = repo
                .org
                .as_ref()
                .map(|o| args.org.contains(o))
                .unwrap_or(false);
            if !matches_org {
                continue;
            }
        }

        if is_uncommitted {
            uncommitted_count += 1;
        }
        if is_behind {
            behind_count += 1;
        }

        // Build display name: "org/name" or just the path's last segment
        let name = repo
            .path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("?");
        let full_name = match &repo.org {
            Some(org) => format!("{}/{}", org, name),
            None => name.to_string(),
        };

        if args.detailed {
            println!("{}", full_name);
            println!("  Branch: {}", repo.current_branch);
            if repo.ahead > 0 || repo.behind > 0 {
                println!("  Ahead: {}, Behind: {}", repo.ahead, repo.behind);
            }
            if repo.staged_count > 0 || repo.unstaged_count > 0 {
                println!("  Status: uncommitted changes");
            }
            if repo.untracked_count > 0 {
                println!("  Status: has untracked files");
            }
        } else {
            let mut indicators = Vec::new();
            if is_uncommitted {
                indicators.push("*".to_string());
            }
            if repo.ahead > 0 {
                indicators.push(format!("+{}", repo.ahead));
            }
            if repo.behind > 0 {
                indicators.push(format!("-{}", repo.behind));
            }

            if indicators.is_empty() {
                println!("  {} (clean)", full_name);
            } else {
                println!("  {} [{}]", full_name, indicators.join(", "));
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
    } else if uncommitted_count == 0 {
        output.success("All repositories are clean and up to date");
    }

    Ok(())
}

#[cfg(test)]
#[path = "status_tests.rs"]
mod tests;
