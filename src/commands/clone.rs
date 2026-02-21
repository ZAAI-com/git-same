//! Clone command handler.

use super::{expand_path, warn_if_concurrency_capped};
use crate::adapters::auth::get_auth;
use crate::adapters::cache::{CacheManager, DiscoveryCache};
use crate::adapters::config::Config;
use crate::adapters::git::{CloneOptions, ShellGit};
use crate::adapters::output::{
    format_count, CloneProgressBar, DiscoveryProgressBar, Output, Verbosity,
};
use crate::adapters::provider::create_provider;
use crate::cli::CloneArgs;
use crate::core::operations::clone::{CloneManager, CloneManagerOptions, CloneProgress};
use crate::discovery::DiscoveryOrchestrator;
use crate::errors::{AppError, Result};
use std::sync::Arc;

/// Clone repositories.
pub async fn run(args: &CloneArgs, config: &Config, output: &Output) -> Result<()> {
    let verbosity = if output.is_json() {
        Verbosity::Quiet
    } else {
        output.verbosity()
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

    // Apply CLI filter overrides
    if !args.org.is_empty() {
        filters.orgs = args.org.clone();
    }
    if args.include_archived {
        filters.include_archived = true;
    }
    if args.include_forks {
        filters.include_forks = true;
    }

    let orchestrator = DiscoveryOrchestrator::new(filters, config.structure.clone());

    // Check cache unless --no-cache or --refresh
    let mut repos = Vec::new();
    let use_cache = !args.no_cache;
    let force_refresh = args.refresh;

    if use_cache && !force_refresh {
        if let Ok(cache_manager) = CacheManager::new() {
            if let Ok(Some(cache)) = cache_manager.load() {
                output.verbose(&format!(
                    "Using cached discovery ({} repos, {} seconds old)",
                    cache.repo_count,
                    cache.age_secs()
                ));
                // Extract repos from cache
                for provider_repos in cache.repos.values() {
                    repos.extend(provider_repos.clone());
                }
            }
        }
    }

    // If no cache or forced refresh, discover from API
    if repos.is_empty() {
        output.info("Discovering repositories...");
        let progress_bar = DiscoveryProgressBar::new(verbosity);
        repos = orchestrator
            .discover(provider.as_ref(), &progress_bar)
            .await?;
        progress_bar.finish();

        // Save to cache unless --no-cache
        if use_cache {
            if let Ok(cache_manager) = CacheManager::new() {
                let mut repos_by_provider = std::collections::HashMap::new();
                let provider_name = provider_entry
                    .name
                    .clone()
                    .unwrap_or_else(|| provider_entry.kind.to_string());
                repos_by_provider.insert(provider_name, repos.clone());
                let cache = DiscoveryCache::new(
                    auth.username.clone().unwrap_or_default(),
                    repos_by_provider,
                );
                if let Err(e) = cache_manager.save(&cache) {
                    output.verbose(&format!("Warning: Failed to save discovery cache: {}", e));
                }
            }
        }
    }

    if repos.is_empty() {
        output.warn("No repositories found matching filters");
        return Ok(());
    }

    output.info(&format_count(repos.len(), "repositories discovered"));

    // Create base path
    let base_path = expand_path(&args.base_path);
    if !base_path.exists() {
        std::fs::create_dir_all(&base_path)
            .map_err(|e| AppError::path(format!("Failed to create base directory: {}", e)))?;
    }

    // Plan clone operation
    let git = ShellGit::new();
    let plan = orchestrator.plan_clone(&base_path, repos, "github", &git);

    if plan.is_empty() && plan.skipped.is_empty() {
        output.success("All repositories already cloned");
        return Ok(());
    }

    // Show plan summary
    if !plan.to_clone.is_empty() {
        output.info(&format_count(plan.to_clone.len(), "repositories to clone"));
    }
    if !plan.to_sync.is_empty() {
        output.info(&format_count(
            plan.to_sync.len(),
            "repositories already exist",
        ));
    }
    if !plan.skipped.is_empty() {
        output.verbose(&format_count(plan.skipped.len(), "repositories skipped"));
    }

    if args.dry_run {
        output.info("Dry run - no changes made");
        for repo in &plan.to_clone {
            println!("  Would clone: {}", repo.full_name());
        }
        return Ok(());
    }

    if plan.to_clone.is_empty() {
        output.success("No new repositories to clone");
        return Ok(());
    }

    // Create clone manager
    let clone_options = CloneOptions {
        depth: args.depth.unwrap_or(config.clone.depth),
        // CLI args override config
        branch: args.branch.clone().or_else(|| {
            if config.clone.branch.is_empty() {
                None
            } else {
                Some(config.clone.branch.clone())
            }
        }),
        recurse_submodules: args.recurse_submodules || config.clone.recurse_submodules,
    };

    let requested_concurrency = args.concurrency.unwrap_or(config.concurrency);
    let effective_concurrency = warn_if_concurrency_capped(requested_concurrency, output);

    let manager_options = CloneManagerOptions::new()
        .with_concurrency(effective_concurrency)
        .with_clone_options(clone_options)
        .with_structure(config.structure.clone())
        .with_ssh(!args.https);

    let manager = CloneManager::new(git, manager_options);

    // Execute clone
    let progress = Arc::new(CloneProgressBar::new(plan.to_clone.len(), verbosity));
    let progress_dyn: Arc<dyn CloneProgress> = progress.clone();
    let (summary, _results) = manager
        .clone_repos(&base_path, plan.to_clone, "github", progress_dyn)
        .await;
    progress.finish(summary.success, summary.failed, summary.skipped);

    // Report results
    if summary.has_failures() {
        output.warn(&format!("{} repositories failed to clone", summary.failed));
    } else {
        output.success(&format!(
            "Successfully cloned {} repositories",
            summary.success
        ));
    }

    Ok(())
}
