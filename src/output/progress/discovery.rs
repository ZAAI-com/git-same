use crate::output::Verbosity;
use crate::provider::DiscoveryProgress;
use console::style;
use indicatif::{MultiProgress, ProgressBar};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use super::styles::spinner_style;

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
