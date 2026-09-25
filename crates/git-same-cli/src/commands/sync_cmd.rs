//! Sync command handler.
//!
//! Combined operation: discover repos -> clone new ones -> fetch/pull existing ones.

use super::warn_if_concurrency_capped;
use crate::cli::SyncCmdArgs;
use git_same_core::config::{Config, WorkspaceManager};
use git_same_core::errors::Result;
use git_same_core::operations::sync::SyncMode;
use git_same_core::output::{
    format_count, format_error, format_skipped, format_success, format_warning, CloneProgressBar,
    DiscoveryProgressBar, Output, SyncProgressBar, Verbosity,
};
use git_same_core::types::{OpSummary, OwnedRepo};
use git_same_core::workflows::sync_workspace::{
    execute_prepared_clone, execute_prepared_fetch, prepare_sync_workspace, SyncExecutionOutcome,
    SyncWorkspaceRequest,
};
use std::sync::Arc;

/// Summary lines printed once both phase bars are done: one per phase that
/// ran, plus a count of repositories skipped at planning.
fn summary_lines(
    outcome: &SyncExecutionOutcome,
    operation: &str,
    skipped_at_planning: usize,
) -> Vec<String> {
    let mut lines = Vec::new();

    if let Some(summary) = &outcome.clone_summary {
        lines.push(phase_line(summary, "Cloned", "new repositories", ""));
    }

    if let Some(summary) = &outcome.sync_summary {
        let with_updates = outcome
            .sync_results
            .iter()
            .filter(|r| r.had_updates && r.result.is_success())
            .count();
        lines.push(phase_line(
            summary,
            &format!("{}ed", operation),
            "repositories",
            &format!(" ({} with updates)", with_updates),
        ));
    }

    if skipped_at_planning > 0 {
        lines.push(format_skipped(&format!(
            "Skipped {} repositories",
            skipped_at_planning
        )));
    } else if lines.is_empty() {
        lines.push(format_success("All repositories are up to date"));
    }

    lines
}

/// One phase's line, e.g. "Fetched 159 of 160 repositories (3 with updates),
/// 1 failed". The "of N" part only appears when something did not succeed.
fn phase_line(summary: &OpSummary, verb: &str, noun: &str, detail: &str) -> String {
    let count = if summary.success == summary.total() {
        summary.success.to_string()
    } else {
        format!("{} of {}", summary.success, summary.total())
    };
    let mut text = format!("{} {} {}{}", verb, count, noun, detail);
    if summary.failed > 0 {
        text.push_str(&format!(", {} failed", summary.failed));
    }
    if summary.skipped > 0 {
        text.push_str(&format!(", {} skipped", summary.skipped));
    }

    if summary.has_failures() {
        format_warning(&text)
    } else {
        format_success(&text)
    }
}

/// One line per repository that did not sync: each failed clone and fetch
/// (git's error trimmed to its first line), then each repository skipped at
/// planning or during a phase. Empty when everything succeeded.
fn failure_lines(
    outcome: &SyncExecutionOutcome,
    skipped_at_planning: &[(&OwnedRepo, &str)],
) -> Vec<String> {
    let clone_results = outcome.clone_results.iter().map(|r| (&r.repo, &r.result));
    let sync_results = outcome.sync_results.iter().map(|r| (&r.repo, &r.result));
    let results: Vec<_> = clone_results.chain(sync_results).collect();

    let failed = results.iter().filter_map(|(repo, result)| {
        let error = result.error_message()?;
        Some(format_error(&repo_line(repo, error)))
    });
    let skipped_during_phase = results.iter().filter_map(|(repo, result)| {
        let reason = result.skip_reason()?;
        Some(format_skipped(&repo_line(repo, reason)))
    });
    let skipped_before = skipped_at_planning
        .iter()
        .map(|(repo, reason)| format_skipped(&repo_line(repo, reason)));

    failed
        .chain(skipped_before)
        .chain(skipped_during_phase)
        .map(|line| format!("  {}", line))
        .collect()
}

/// "owner/repo: <first non-empty line of message>".
fn repo_line(repo: &OwnedRepo, message: &str) -> String {
    let first = message
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("(no details)");
    format!("{}: {}", repo.full_name(), first)
}

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
    // Finish and release the spinner before any phase bar draws below it.
    discovery_progress.finish();
    drop(discovery_progress);

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

    let operation = if prepared.sync_mode == SyncMode::Pull {
        "Pull"
    } else {
        "Fetch"
    };

    // One phase at a time. Each bar is created only when its phase has work
    // and is finished and dropped before the next one starts: two live bars
    // redraw over each other's lines and leave stale frames behind.
    let clone_phase = if prepared.plan.to_clone.is_empty() {
        None
    } else {
        let bar = Arc::new(CloneProgressBar::new(
            prepared.plan.to_clone.len(),
            verbosity,
        ));
        let phase = execute_prepared_clone(&prepared, bar.clone()).await;
        if let Some((summary, _)) = &phase {
            bar.finish(summary.success, summary.failed, summary.skipped);
        }
        phase
    };

    let fetch_phase = if prepared.to_sync.is_empty() {
        None
    } else {
        let bar = Arc::new(SyncProgressBar::new(
            prepared.to_sync.len(),
            verbosity,
            operation,
        ));
        let phase = execute_prepared_fetch(&prepared, bar.clone()).await;
        if let Some((summary, _)) = &phase {
            bar.finish(summary.success, summary.failed, summary.skipped);
        }
        phase
    };

    // Summary, failures and skips print at the default level; only -q and
    // --json hide them.
    let outcome = SyncExecutionOutcome::from_phases(clone_phase, fetch_phase);
    let skipped = prepared.skipped_at_planning();
    for line in summary_lines(&outcome, operation, skipped.len()) {
        output.summary(&line);
    }
    for line in failure_lines(&outcome, &skipped) {
        output.summary(&line);
    }

    // Update last_synced
    workspace.last_synced = Some(chrono::Utc::now().to_rfc3339());
    if let Err(e) = WorkspaceManager::save(&workspace) {
        output.verbose(&format!("Warning: Failed to update last_synced: {}", e));
    }

    // Best-effort: nudge the Finder monitor so badges refresh for new clones.
    // If the monitor is not running we silently skip; sync still succeeded.
    git_same_core::ipc::nudge_refresh_all().await;

    Ok(())
}

#[cfg(test)]
#[path = "sync_cmd_tests.rs"]
mod tests;
