use crate::operations::clone::CloneProgress;
use crate::output::Verbosity;
use crate::types::OwnedRepo;
use console::style;
use indicatif::{MultiProgress, ProgressBar};

use super::styles::progress_style;

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

#[cfg(test)]
#[path = "clone_tests.rs"]
mod tests;
