//! Fetch/Pull command handler.

use super::{expand_path, warn_if_concurrency_capped};
use crate::auth::get_auth;
use crate::cli::LegacySyncArgs;
use crate::config::Config;
use crate::discovery::DiscoveryOrchestrator;
use crate::errors::{AppError, Result};
use crate::git::ShellGit;
use crate::operations::sync::{SyncManager, SyncManagerOptions, SyncMode, SyncProgress};
use crate::output::{format_count, DiscoveryProgressBar, Output, SyncProgressBar, Verbosity};
use crate::provider::create_provider;
use std::sync::Arc;

/// Sync (fetch or pull) repositories.
pub async fn run(
    args: &LegacySyncArgs,
    config: &Config,
    output: &Output,
    mode: SyncMode,
) -> Result<()> {
    let verbosity = if output.is_json() {
        Verbosity::Quiet
    } else {
        output.verbosity()
    };
    let operation = if mode == SyncMode::Pull {
        "Pull"
    } else {
        "Fetch"
    };

    // Get authentication
    output.info("Authenticating...");
    let auth = get_auth(None)?;
    output.verbose(&format!(
        "Authenticated as {:?} via {}",
        auth.username, auth.method
    ));

    // Get first enabled provider from config
    let provider_entry = config
        .enabled_providers()
        .next()
        .ok_or_else(|| AppError::config("No enabled providers configured"))?;

    // Create provider
    let provider = create_provider(provider_entry, &auth.token)?;

    // Create discovery orchestrator
    let mut filters = config.filters.clone();
    if !args.org.is_empty() {
        filters.orgs = args.org.clone();
    }

    let orchestrator = DiscoveryOrchestrator::new(filters, config.structure.clone());

    // Discover repositories
    output.info("Discovering repositories...");
    let progress_bar = DiscoveryProgressBar::new(verbosity);
    let repos = orchestrator
        .discover(provider.as_ref(), &progress_bar)
        .await?;
    progress_bar.finish();

    if repos.is_empty() {
        output.warn("No repositories found matching filters");
        return Ok(());
    }

    // Expand base path
    let base_path = expand_path(&args.base_path);
    if !base_path.exists() {
        return Err(AppError::config(format!(
            "Base path does not exist: {}",
            base_path.display()
        )));
    }

    // Plan sync operation
    let git = ShellGit::new();
    let skip_uncommitted = !args.no_skip_uncommitted;
    let (to_sync, skipped) = orchestrator.plan_sync(&base_path, repos, "github", &git, skip_uncommitted);

    if to_sync.is_empty() {
        if skipped.is_empty() {
            output.warn("No repositories found to sync");
        } else {
            output.info(&format!("All {} repositories were skipped", skipped.len()));
        }
        return Ok(());
    }

    // Show plan summary
    output.info(&format_count(
        to_sync.len(),
        &format!("repositories to {}", operation.to_lowercase()),
    ));
    if !skipped.is_empty() {
        output.verbose(&format_count(skipped.len(), "repositories skipped"));
    }

    if args.dry_run {
        output.info("Dry run - no changes made");
        for repo in &to_sync {
            println!(
                "  Would {}: {}",
                operation.to_lowercase(),
                repo.repo.full_name()
            );
        }
        return Ok(());
    }

    // Create sync manager
    let requested_concurrency = args.concurrency.unwrap_or(config.concurrency);
    let effective_concurrency = warn_if_concurrency_capped(requested_concurrency, output);

    let manager_options = SyncManagerOptions::new()
        .with_concurrency(effective_concurrency)
        .with_mode(mode)
        .with_skip_uncommitted(skip_uncommitted);

    let manager = SyncManager::new(git, manager_options);

    // Execute sync
    let progress = Arc::new(SyncProgressBar::new(to_sync.len(), verbosity, operation));
    let progress_dyn: Arc<dyn SyncProgress> = progress.clone();
    let (summary, results) = manager.sync_repos(to_sync, progress_dyn).await;
    progress.finish(summary.success, summary.failed, summary.skipped);

    // Count updates
    let with_updates = results.iter().filter(|r| r.had_updates).count();

    // Report results
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

    Ok(())
}

#[cfg(test)]
mod tests {
    // Sync command orchestrates auth -> provider -> discovery -> sync.
    // Unit tests are not feasible because `run()` calls `get_auth(None)?`
    // which requires real credentials (GitHub CLI, env vars, or config token).
    //
    // Component-level tests exist in:
    // - src/operations/sync.rs (SyncManager)
    // - src/discovery/mod.rs (DiscoveryOrchestrator)
    //
    // Integration coverage: tests/integration_test.rs
}
