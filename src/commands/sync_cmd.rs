//! Sync command handler.
//!
//! Combined operation: discover repos -> clone new ones -> fetch/pull existing ones.

use super::warn_if_concurrency_capped;
use crate::cli::SyncCmdArgs;
use crate::config::{Config, WorkspaceManager};
use crate::errors::Result;
use crate::operations::clone::CloneProgress;
use crate::operations::sync::{SyncMode, SyncProgress};
use crate::output::{
    format_count, CloneProgressBar, DiscoveryProgressBar, Output, SyncProgressBar, Verbosity,
};
use crate::workflows::sync_workspace::{
    execute_prepared_sync, prepare_sync_workspace, SyncWorkspaceRequest,
};
use std::sync::Arc;

/// Sync repositories for a workspace.
pub async fn run(args: &SyncCmdArgs, config: &Config, output: &Output) -> Result<()> {
    let verbosity = if output.is_json() {
        Verbosity::Quiet
    } else {
        output.verbosity()
    };

    // Resolve workspace and ensure base path exists (offer to fix if user moved it)
    let mut workspace = WorkspaceManager::resolve(args.workspace.as_deref(), config)?;
    super::ensure_base_path(&workspace, output)?;

    output.info("Discovering repositories...");
    let discovery_progress = DiscoveryProgressBar::new(verbosity);
    let prepared = prepare_sync_workspace(
        SyncWorkspaceRequest {
            config,
            workspace: &workspace,
            refresh: args.refresh,
            skip_uncommitted: !args.no_skip_uncommitted,
            pull: args.pull,
            concurrency_override: args.concurrency,
            create_base_path: false,
        },
        &discovery_progress,
    )
    .await?;
    discovery_progress.finish();

    output.verbose(&format!(
        "Authenticated as {:?} via {}",
        prepared.auth.username, prepared.auth.method
    ));

    if prepared.used_cache {
        if let Some(age_secs) = prepared.cache_age_secs {
            output.verbose(&format!(
                "Using cached discovery ({} repos, {} seconds old)",
                prepared.repos.len(),
                age_secs
            ));
        }
    }

    if prepared.repos.is_empty() {
        output.warn("No repositories found matching filters");
        return Ok(());
    }

    output.info(&format_count(
        prepared.repos.len(),
        "repositories discovered",
    ));

    let effective_concurrency = warn_if_concurrency_capped(prepared.requested_concurrency, output);
    debug_assert_eq!(effective_concurrency, prepared.effective_concurrency);

    // Dry-run output
    let had_clones = !prepared.plan.to_clone.is_empty();
    if args.dry_run {
        if had_clones {
            output.info(&format!(
                "Would clone {} new repositories:",
                prepared.plan.to_clone.len()
            ));
            for repo in &prepared.plan.to_clone {
                output.info(&format!("  + {}", repo.full_name()));
            }
        }

        if !prepared.to_sync.is_empty() {
            let op = if prepared.sync_mode == SyncMode::Pull {
                "pull"
            } else {
                "fetch"
            };
            output.info(&format!(
                "Would {} {} existing repositories:",
                op,
                prepared.to_sync.len()
            ));
            for repo in &prepared.to_sync {
                output.info(&format!("  ~ {}", repo.repo.full_name()));
            }
        } else if !had_clones {
            output.success("All repositories are up to date");
        }

        return Ok(());
    }

    // Execute shared workflow
    let clone_progress = Arc::new(CloneProgressBar::new(
        prepared.plan.to_clone.len(),
        verbosity,
    ));
    let clone_progress_dyn: Arc<dyn CloneProgress> = clone_progress.clone();

    let operation = if prepared.sync_mode == SyncMode::Pull {
        "Pull"
    } else {
        "Fetch"
    };
    let sync_progress = Arc::new(SyncProgressBar::new(
        prepared.to_sync.len(),
        verbosity,
        operation,
    ));
    let sync_progress_dyn: Arc<dyn SyncProgress> = sync_progress.clone();

    let outcome =
        execute_prepared_sync(&prepared, false, clone_progress_dyn, sync_progress_dyn).await;

    if let Some(summary) = &outcome.clone_summary {
        clone_progress.finish(summary.success, summary.failed, summary.skipped);
        if summary.has_failures() {
            output.warn(&format!("{} repositories failed to clone", summary.failed));
        } else if summary.success > 0 {
            output.success(&format!("Cloned {} new repositories", summary.success));
        }
    }

    if let Some(summary) = &outcome.sync_summary {
        sync_progress.finish(summary.success, summary.failed, summary.skipped);

        let with_updates = outcome
            .sync_results
            .iter()
            .filter(|r| r.had_updates)
            .count();
        if summary.has_failures() {
            output.warn(&format!(
                "{} of {} repositories failed to {}",
                summary.failed,
                summary.total(),
                operation.to_lowercase()
            ));
        } else {
            output.success(&format!(
                "{}ed {} repositories ({} with updates)",
                operation, summary.success, with_updates
            ));
        }
    } else if !had_clones {
        output.success("All repositories are up to date");
    }

    // Update last_synced
    workspace.last_synced = Some(chrono::Utc::now().to_rfc3339());
    if let Err(e) = WorkspaceManager::save(&workspace) {
        output.verbose(&format!("Warning: Failed to update last_synced: {}", e));
    }

    Ok(())
}

#[cfg(test)]
#[path = "sync_cmd_tests.rs"]
mod tests;
