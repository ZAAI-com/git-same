use crate::git::{FetchResult, PullResult};
use crate::operations::sync::SyncProgress;
use crate::output::Verbosity;
use crate::types::OwnedRepo;
use console::style;
use indicatif::{MultiProgress, ProgressBar};
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use super::styles::progress_style;

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
        result: &PullResult,
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
