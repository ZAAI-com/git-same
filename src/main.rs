//! Gisa - Mirror GitHub org/repo structure locally
//!
//! Main entry point for the gisa CLI application.

use gisa::auth::get_auth;
use gisa::cli::{Cli, CloneArgs, Command, InitArgs, StatusArgs, SyncArgs};
use gisa::clone::{CloneManager, CloneManagerOptions, NoProgress as NoCloneProgress};
use gisa::config::{Config, SyncMode as ConfigSyncMode};
use gisa::discovery::DiscoveryOrchestrator;
use gisa::errors::{AppError, Result};
use gisa::git::ShellGit;
use gisa::output::{
    format_count, format_error, format_success, CloneProgressBar, DiscoveryProgressBar, Output,
    SyncProgressBar, Verbosity,
};
use gisa::provider::{create_provider, Credentials, NoProgress as NoDiscoveryProgress};
use gisa::sync::{LocalRepo, NoSyncProgress, SyncManager, SyncManagerOptions, SyncMode};
use std::path::PathBuf;
use std::process::ExitCode;

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse_args();

    // Create output handler
    let verbosity = Verbosity::from(cli.verbosity());
    let output = Output::new(verbosity, cli.is_json());

    // Run command and handle result
    let result = run_command(&cli, &output).await;

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            output.error(&e.to_string());
            if verbosity >= Verbosity::Verbose {
                if let Some(action) = e.suggested_action() {
                    eprintln!("  Suggestion: {}", action);
                }
            }
            ExitCode::from(e.exit_code())
        }
    }
}

/// Run the specified command.
async fn run_command(cli: &Cli, output: &Output) -> Result<()> {
    // Load config
    let config = if let Some(ref path) = cli.config {
        Config::load_from(path)?
    } else {
        Config::load()?
    };

    match &cli.command {
        Command::Init(args) => cmd_init(args, output).await,
        Command::Clone(args) => cmd_clone(args, &config, output).await,
        Command::Fetch(args) => cmd_sync(args, &config, output, SyncMode::Fetch).await,
        Command::Pull(args) => cmd_sync(args, &config, output, SyncMode::Pull).await,
        Command::Status(args) => cmd_status(args, &config, output).await,
        Command::Completions(args) => {
            gisa::cli::generate_completions(args.shell);
            Ok(())
        }
    }
}

/// Initialize gisa configuration.
async fn cmd_init(args: &InitArgs, output: &Output) -> Result<()> {
    let config_path = args.path.clone().unwrap_or_else(Config::default_path);

    // Check if config already exists
    if config_path.exists() && !args.force {
        return Err(AppError::config(format!(
            "Config file already exists at {}. Use --force to overwrite.",
            config_path.display()
        )));
    }

    // Create parent directory
    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            AppError::io(format!("Failed to create config directory: {}", e))
        })?;
    }

    // Write default config
    let default_config = Config::default_toml();
    std::fs::write(&config_path, default_config)
        .map_err(|e| AppError::io(format!("Failed to write config: {}", e)))?;

    output.success(&format!("Created config at {}", config_path.display()));
    output.info("Edit this file to customize gisa behavior");
    output.info("Run 'gisa clone <path>' to clone your repositories");

    Ok(())
}

/// Clone repositories.
async fn cmd_clone(args: &CloneArgs, config: &Config, output: &Output) -> Result<()> {
    let verbosity = Verbosity::from(if output.is_json() { 0 } else { 1 });

    // Get authentication
    output.info("Authenticating...");
    let auth = get_auth(None)?;
    output.verbose(&format!("Authenticated as {:?} via {}", auth.username, auth.method));

    // Create provider
    let credentials = Credentials::new(auth.token);
    let provider = create_provider(gisa::types::ProviderKind::GitHub, credentials, None)?;

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

    // Discover repositories
    output.info("Discovering repositories...");
    let progress_bar = DiscoveryProgressBar::new(verbosity);
    let repos = orchestrator.discover(provider.as_ref(), &progress_bar).await?;
    progress_bar.finish();

    if repos.is_empty() {
        output.warn("No repositories found matching filters");
        return Ok(());
    }

    output.info(&format_count(repos.len(), "repositories discovered"));

    // Create base path
    let base_path = expand_path(&args.base_path);
    if !base_path.exists() {
        std::fs::create_dir_all(&base_path).map_err(|e| {
            AppError::io(format!("Failed to create base directory: {}", e))
        })?;
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
        output.info(&format_count(plan.to_sync.len(), "repositories already exist"));
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
    let clone_options = gisa::git::CloneOptions {
        depth: args.depth.unwrap_or(config.clone.depth),
        branch: if config.clone.branch.is_empty() {
            None
        } else {
            Some(config.clone.branch.clone())
        },
        recurse_submodules: config.clone.recurse_submodules,
    };

    let manager_options = CloneManagerOptions::new()
        .with_concurrency(args.concurrency.unwrap_or(config.concurrency))
        .with_clone_options(clone_options)
        .with_structure(config.structure.clone())
        .with_ssh(!args.https);

    let manager = CloneManager::new(git, manager_options);

    // Execute clone
    let progress = CloneProgressBar::new(plan.to_clone.len(), verbosity);
    let (summary, _results) = manager
        .clone_repos(&base_path, plan.to_clone, "github", &progress)
        .await;
    progress.finish(summary.success, summary.failed, summary.skipped);

    // Report results
    if summary.has_failures() {
        output.warn(&format!(
            "{} repositories failed to clone",
            summary.failed
        ));
    } else {
        output.success(&format!(
            "Successfully cloned {} repositories",
            summary.success
        ));
    }

    Ok(())
}

/// Sync (fetch or pull) repositories.
async fn cmd_sync(
    args: &SyncArgs,
    config: &Config,
    output: &Output,
    mode: SyncMode,
) -> Result<()> {
    let verbosity = Verbosity::from(if output.is_json() { 0 } else { 1 });
    let operation = if mode == SyncMode::Pull { "Pull" } else { "Fetch" };

    // Get authentication
    output.info("Authenticating...");
    let auth = get_auth(None)?;
    output.verbose(&format!("Authenticated as {:?} via {}", auth.username, auth.method));

    // Create provider
    let credentials = Credentials::new(auth.token);
    let provider = create_provider(gisa::types::ProviderKind::GitHub, credentials, None)?;

    // Create discovery orchestrator
    let mut filters = config.filters.clone();
    if !args.org.is_empty() {
        filters.orgs = args.org.clone();
    }

    let orchestrator = DiscoveryOrchestrator::new(filters, config.structure.clone());

    // Discover repositories
    output.info("Discovering repositories...");
    let progress_bar = DiscoveryProgressBar::new(verbosity);
    let repos = orchestrator.discover(provider.as_ref(), &progress_bar).await?;
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
    let (to_sync, skipped) =
        orchestrator.plan_sync(&base_path, repos, "github", &git, args.skip_dirty);

    if to_sync.is_empty() {
        if skipped.is_empty() {
            output.warn("No repositories found to sync");
        } else {
            output.info(&format!(
                "All {} repositories were skipped",
                skipped.len()
            ));
        }
        return Ok(());
    }

    // Show plan summary
    output.info(&format_count(to_sync.len(), &format!("repositories to {}", operation.to_lowercase())));
    if !skipped.is_empty() {
        output.verbose(&format_count(skipped.len(), "repositories skipped"));
    }

    if args.dry_run {
        output.info("Dry run - no changes made");
        for repo in &to_sync {
            println!("  Would {}: {}", operation.to_lowercase(), repo.repo.full_name());
        }
        return Ok(());
    }

    // Create sync manager
    let manager_options = SyncManagerOptions::new()
        .with_concurrency(args.concurrency.unwrap_or(config.concurrency))
        .with_mode(mode)
        .with_skip_dirty(args.skip_dirty);

    let manager = SyncManager::new(git, manager_options);

    // Execute sync
    let progress = SyncProgressBar::new(to_sync.len(), verbosity, operation);
    let (summary, results) = manager.sync_repos(to_sync, &progress).await;
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
            operation,
            summary.success,
            with_updates
        ));
    }

    Ok(())
}

/// Show status of repositories.
async fn cmd_status(args: &StatusArgs, config: &Config, output: &Output) -> Result<()> {
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
                output.verbose(&format!("  {} - error: {}", format!("{}/{}", org, name), e));
            }
        }
    }

    // Summary
    println!();
    if dirty_count > 0 {
        output.warn(&format!("{} repositories have uncommitted changes", dirty_count));
    }
    if behind_count > 0 {
        output.info(&format!("{} repositories are behind upstream", behind_count));
    }
    if dirty_count == 0 && behind_count == 0 {
        output.success("All repositories are clean and up to date");
    }

    Ok(())
}

/// Expands ~ and environment variables in a path.
fn expand_path(path: &PathBuf) -> PathBuf {
    let path_str = path.to_string_lossy();
    let expanded = shellexpand::tilde(&path_str);
    PathBuf::from(expanded.as_ref())
}
