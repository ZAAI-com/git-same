//! Progress reporting utilities using indicatif.
//!
//! This module provides progress bars and status reporting for long-running operations.

use crate::clone::CloneProgress;
use crate::git::FetchResult;
use crate::provider::DiscoveryProgress;
use crate::sync::SyncProgress;
use crate::types::OwnedRepo;
use console::style;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

/// Default spinner style frames.
const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

/// Creates a default spinner style.
pub fn spinner_style() -> ProgressStyle {
    ProgressStyle::default_spinner()
        .tick_strings(SPINNER_FRAMES)
        .template("{spinner:.cyan} {msg}")
        .expect("Invalid spinner template")
}

/// Creates a progress bar style.
pub fn progress_style() -> ProgressStyle {
    ProgressStyle::default_bar()
        .template("{spinner:.cyan} [{bar:40.cyan/dim}] {pos}/{len} {msg}")
        .expect("Invalid progress template")
        .progress_chars("━╸─")
}

/// Creates a progress bar style with rate.
pub fn progress_style_with_rate() -> ProgressStyle {
    ProgressStyle::default_bar()
        .template("{spinner:.cyan} [{bar:40.cyan/dim}] {pos}/{len} ({per_sec}) {msg}")
        .expect("Invalid progress template")
        .progress_chars("━╸─")
}

/// Output verbosity level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Verbosity {
    /// No output except errors
    Quiet = 0,
    /// Normal output
    Normal = 1,
    /// Verbose output
    Verbose = 2,
    /// Very verbose (debug) output
    Debug = 3,
}

impl From<u8> for Verbosity {
    fn from(v: u8) -> Self {
        match v {
            0 => Verbosity::Quiet,
            1 => Verbosity::Normal,
            2 => Verbosity::Verbose,
            _ => Verbosity::Debug,
        }
    }
}

/// Output handler for consistent formatting.
#[derive(Debug, Clone)]
pub struct Output {
    verbosity: Verbosity,
    json: bool,
}

impl Output {
    /// Creates a new output handler.
    pub fn new(verbosity: Verbosity, json: bool) -> Self {
        Self { verbosity, json }
    }

    /// Creates a quiet output handler.
    pub fn quiet() -> Self {
        Self::new(Verbosity::Quiet, false)
    }

    /// Prints an info message.
    pub fn info(&self, msg: &str) {
        if !self.json && self.verbosity >= Verbosity::Normal {
            println!("{} {}", style("→").cyan(), msg);
        }
    }

    /// Prints a success message.
    pub fn success(&self, msg: &str) {
        if !self.json && self.verbosity >= Verbosity::Normal {
            println!("{} {}", style("✓").green(), msg);
        }
    }

    /// Prints a warning message.
    pub fn warn(&self, msg: &str) {
        if !self.json && self.verbosity >= Verbosity::Normal {
            eprintln!("{} {}", style("⚠").yellow(), msg);
        }
    }

    /// Prints an error message.
    pub fn error(&self, msg: &str) {
        if !self.json {
            eprintln!("{} {}", style("✗").red(), msg);
        }
    }

    /// Prints a verbose message.
    pub fn verbose(&self, msg: &str) {
        if !self.json && self.verbosity >= Verbosity::Verbose {
            println!("{} {}", style("·").dim(), msg);
        }
    }

    /// Prints a debug message.
    pub fn debug(&self, msg: &str) {
        if !self.json && self.verbosity >= Verbosity::Debug {
            println!("{} {}", style("⋅").dim(), style(msg).dim());
        }
    }

    /// Returns true if output is in JSON mode.
    pub fn is_json(&self) -> bool {
        self.json
    }
}

impl Default for Output {
    fn default() -> Self {
        Self::new(Verbosity::Normal, false)
    }
}

/// Progress reporter for discovery operations.
pub struct DiscoveryProgressBar {
    #[allow(dead_code)]
    multi: MultiProgress,
    main_bar: ProgressBar,
    repo_count: Arc<AtomicUsize>,
    verbosity: Verbosity,
}

impl DiscoveryProgressBar {
    /// Creates a new discovery progress bar.
    pub fn new(verbosity: Verbosity) -> Self {
        let multi = MultiProgress::new();
        let main_bar = multi.add(ProgressBar::new_spinner());
        main_bar.set_style(spinner_style());
        main_bar.set_message("Discovering repositories...");
        main_bar.enable_steady_tick(std::time::Duration::from_millis(100));

        Self {
            multi,
            main_bar,
            repo_count: Arc::new(AtomicUsize::new(0)),
            verbosity,
        }
    }

    /// Finishes the progress bar.
    pub fn finish(&self) {
        let count = self.repo_count.load(Ordering::SeqCst);
        self.main_bar.finish_with_message(format!(
            "{} Discovered {} repositories",
            style("✓").green(),
            count
        ));
    }
}

impl DiscoveryProgress for DiscoveryProgressBar {
    fn on_orgs_discovered(&self, count: usize) {
        if self.verbosity >= Verbosity::Verbose {
            self.main_bar
                .set_message(format!("Found {} organizations", count));
        }
    }

    fn on_org_started(&self, org_name: &str) {
        if self.verbosity >= Verbosity::Verbose {
            self.main_bar
                .set_message(format!("Discovering: {}", style(org_name).cyan()));
        }
    }

    fn on_org_complete(&self, org_name: &str, repo_count: usize) {
        self.repo_count.fetch_add(repo_count, Ordering::SeqCst);
        let total = self.repo_count.load(Ordering::SeqCst);
        self.main_bar.set_message(format!(
            "Discovered {} repos ({} from {})",
            total,
            repo_count,
            style(org_name).cyan()
        ));
    }

    fn on_personal_repos_started(&self) {
        if self.verbosity >= Verbosity::Verbose {
            self.main_bar
                .set_message("Discovering personal repositories...");
        }
    }

    fn on_personal_repos_complete(&self, count: usize) {
        self.repo_count.fetch_add(count, Ordering::SeqCst);
        let total = self.repo_count.load(Ordering::SeqCst);
        self.main_bar
            .set_message(format!("Discovered {} repos (including personal)", total));
    }

    fn on_error(&self, message: &str) {
        if self.verbosity >= Verbosity::Normal {
            self.main_bar.suspend(|| {
                eprintln!("{} {}", style("⚠").yellow(), message);
            });
        }
    }
}

/// Progress reporter for clone operations.
pub struct CloneProgressBar {
    #[allow(dead_code)]
    multi: MultiProgress,
    main_bar: ProgressBar,
    verbosity: Verbosity,
}

impl CloneProgressBar {
    /// Creates a new clone progress bar.
    pub fn new(total: usize, verbosity: Verbosity) -> Self {
        let multi = MultiProgress::new();
        let main_bar = multi.add(ProgressBar::new(total as u64));
        main_bar.set_style(progress_style());
        main_bar.set_message("Cloning repositories...");
        main_bar.enable_steady_tick(std::time::Duration::from_millis(100));

        Self {
            multi,
            main_bar,
            verbosity,
        }
    }

    /// Finishes the progress bar.
    pub fn finish(&self, success: usize, failed: usize, skipped: usize) {
        let msg = format!(
            "{} {} cloned, {} failed, {} skipped",
            style("✓").green(),
            success,
            failed,
            skipped
        );
        self.main_bar.finish_with_message(msg);
    }
}

impl CloneProgress for CloneProgressBar {
    fn on_start(&self, repo: &OwnedRepo, _index: usize, _total: usize) {
        if self.verbosity >= Verbosity::Verbose {
            self.main_bar
                .set_message(format!("Cloning {}...", style(repo.full_name()).cyan()));
        }
    }

    fn on_complete(&self, repo: &OwnedRepo, _index: usize, _total: usize) {
        self.main_bar.inc(1);
        if self.verbosity >= Verbosity::Debug {
            self.main_bar.suspend(|| {
                println!("{} Cloned {}", style("✓").green(), repo.full_name());
            });
        }
    }

    fn on_error(&self, repo: &OwnedRepo, error: &str, _index: usize, _total: usize) {
        self.main_bar.inc(1);
        if self.verbosity >= Verbosity::Normal {
            self.main_bar.suspend(|| {
                eprintln!(
                    "{} Failed to clone {}: {}",
                    style("✗").red(),
                    repo.full_name(),
                    error
                );
            });
        }
    }

    fn on_skip(&self, repo: &OwnedRepo, reason: &str, _index: usize, _total: usize) {
        self.main_bar.inc(1);
        if self.verbosity >= Verbosity::Verbose {
            self.main_bar.suspend(|| {
                println!(
                    "{} Skipped {}: {}",
                    style("→").dim(),
                    repo.full_name(),
                    reason
                );
            });
        }
    }
}

/// Progress reporter for sync operations.
pub struct SyncProgressBar {
    #[allow(dead_code)]
    multi: MultiProgress,
    main_bar: ProgressBar,
    verbosity: Verbosity,
    updates_count: Arc<AtomicUsize>,
}

impl SyncProgressBar {
    /// Creates a new sync progress bar.
    pub fn new(total: usize, verbosity: Verbosity, operation: &str) -> Self {
        let multi = MultiProgress::new();
        let main_bar = multi.add(ProgressBar::new(total as u64));
        main_bar.set_style(progress_style());
        main_bar.set_message(format!("{}ing repositories...", operation));
        main_bar.enable_steady_tick(std::time::Duration::from_millis(100));

        Self {
            multi,
            main_bar,
            verbosity,
            updates_count: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Finishes the progress bar.
    pub fn finish(&self, success: usize, failed: usize, skipped: usize) {
        let updates = self.updates_count.load(Ordering::SeqCst);
        let msg = format!(
            "{} {} synced ({} with updates), {} failed, {} skipped",
            style("✓").green(),
            success,
            updates,
            failed,
            skipped
        );
        self.main_bar.finish_with_message(msg);
    }
}

impl SyncProgress for SyncProgressBar {
    fn on_start(&self, repo: &OwnedRepo, _path: &Path, _index: usize, _total: usize) {
        if self.verbosity >= Verbosity::Verbose {
            self.main_bar
                .set_message(format!("Syncing {}...", style(repo.full_name()).cyan()));
        }
    }

    fn on_fetch_complete(
        &self,
        repo: &OwnedRepo,
        result: &FetchResult,
        _index: usize,
        _total: usize,
    ) {
        self.main_bar.inc(1);
        if result.updated {
            self.updates_count.fetch_add(1, Ordering::SeqCst);
        }
        if self.verbosity >= Verbosity::Debug {
            let status = if result.updated {
                "updated"
            } else {
                "up to date"
            };
            self.main_bar.suspend(|| {
                println!(
                    "{} {} {}",
                    style("✓").green(),
                    repo.full_name(),
                    style(status).dim()
                );
            });
        }
    }

    fn on_pull_complete(
        &self,
        repo: &OwnedRepo,
        result: &crate::git::PullResult,
        _index: usize,
        _total: usize,
    ) {
        self.main_bar.inc(1);
        if result.success {
            self.updates_count.fetch_add(1, Ordering::SeqCst);
        }
        if self.verbosity >= Verbosity::Debug {
            let status = if result.fast_forward {
                "fast-forward"
            } else {
                "merged"
            };
            self.main_bar.suspend(|| {
                println!(
                    "{} {} {}",
                    style("✓").green(),
                    repo.full_name(),
                    style(status).dim()
                );
            });
        }
    }

    fn on_error(&self, repo: &OwnedRepo, error: &str, _index: usize, _total: usize) {
        self.main_bar.inc(1);
        if self.verbosity >= Verbosity::Normal {
            self.main_bar.suspend(|| {
                eprintln!(
                    "{} Failed to sync {}: {}",
                    style("✗").red(),
                    repo.full_name(),
                    error
                );
            });
        }
    }

    fn on_skip(&self, repo: &OwnedRepo, reason: &str, _index: usize, _total: usize) {
        self.main_bar.inc(1);
        if self.verbosity >= Verbosity::Verbose {
            self.main_bar.suspend(|| {
                println!(
                    "{} Skipped {}: {}",
                    style("→").dim(),
                    repo.full_name(),
                    reason
                );
            });
        }
    }
}

/// Format a count with appropriate styling.
pub fn format_count(count: usize, label: &str) -> String {
    format!("{} {}", style(count).cyan().bold(), label)
}

/// Format a success message.
pub fn format_success(msg: &str) -> String {
    format!("{} {}", style("✓").green(), msg)
}

/// Format an error message.
pub fn format_error(msg: &str) -> String {
    format!("{} {}", style("✗").red(), msg)
}

/// Format a warning message.
pub fn format_warning(msg: &str) -> String {
    format!("{} {}", style("⚠").yellow(), msg)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verbosity_from_u8() {
        assert_eq!(Verbosity::from(0), Verbosity::Quiet);
        assert_eq!(Verbosity::from(1), Verbosity::Normal);
        assert_eq!(Verbosity::from(2), Verbosity::Verbose);
        assert_eq!(Verbosity::from(3), Verbosity::Debug);
        assert_eq!(Verbosity::from(100), Verbosity::Debug);
    }

    #[test]
    fn test_verbosity_ordering() {
        assert!(Verbosity::Quiet < Verbosity::Normal);
        assert!(Verbosity::Normal < Verbosity::Verbose);
        assert!(Verbosity::Verbose < Verbosity::Debug);
    }

    #[test]
    fn test_output_creation() {
        let output = Output::new(Verbosity::Normal, false);
        assert!(!output.is_json());

        let json_output = Output::new(Verbosity::Normal, true);
        assert!(json_output.is_json());
    }

    #[test]
    fn test_output_quiet() {
        let output = Output::quiet();
        assert_eq!(output.verbosity, Verbosity::Quiet);
    }

    #[test]
    fn test_format_functions() {
        // Just verify they don't panic and return strings
        let count = format_count(42, "repos");
        assert!(count.contains("42"));
        assert!(count.contains("repos"));

        let success = format_success("done");
        assert!(success.contains("done"));

        let error = format_error("failed");
        assert!(error.contains("failed"));

        let warning = format_warning("caution");
        assert!(warning.contains("caution"));
    }
}
