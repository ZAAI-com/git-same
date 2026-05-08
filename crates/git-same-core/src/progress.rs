//! Host-neutral progress events and adapters.
//!
//! The engine exposes operation-specific progress traits for discovery, clone,
//! and sync. This module gives hosts a single serializable event stream without
//! replacing those traits, so CLIs, TUIs, and GUI hosts can choose their own
//! presentation layer.

use crate::git::{FetchResult, PullResult};
use crate::operations::clone::CloneProgress;
use crate::operations::sync::SyncProgress;
use crate::provider::DiscoveryProgress;
use crate::types::OwnedRepo;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;

/// A serializable progress event emitted by discovery, clone, and sync work.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ProgressEvent {
    /// Discovery found the list of organizations it will scan.
    DiscoveryOrgsDiscovered { count: usize },
    /// Discovery started fetching repositories for an organization.
    DiscoveryOrgStarted { org_name: String },
    /// Discovery finished fetching repositories for an organization.
    DiscoveryOrgComplete { org_name: String, repo_count: usize },
    /// Discovery started fetching personal repositories.
    DiscoveryPersonalReposStarted,
    /// Discovery finished fetching personal repositories.
    DiscoveryPersonalReposComplete { count: usize },
    /// Discovery encountered a non-fatal error.
    DiscoveryError { message: String },
    /// A clone operation started.
    CloneStarted {
        repo_name: String,
        index: usize,
        total: usize,
    },
    /// A clone operation completed.
    CloneCompleted {
        repo_name: String,
        index: usize,
        total: usize,
    },
    /// A clone operation failed.
    CloneFailed {
        repo_name: String,
        error: String,
        index: usize,
        total: usize,
    },
    /// A clone operation was skipped.
    CloneSkipped {
        repo_name: String,
        reason: String,
        index: usize,
        total: usize,
    },
    /// A sync operation started.
    SyncStarted {
        repo_name: String,
        path: String,
        index: usize,
        total: usize,
    },
    /// A fetch operation completed during sync.
    SyncFetched {
        repo_name: String,
        updated: bool,
        new_commits: Option<u32>,
        index: usize,
        total: usize,
    },
    /// A pull operation completed during sync.
    SyncPulled {
        repo_name: String,
        success: bool,
        updated: bool,
        fast_forward: bool,
        error: Option<String>,
        index: usize,
        total: usize,
    },
    /// A sync operation failed.
    SyncFailed {
        repo_name: String,
        error: String,
        index: usize,
        total: usize,
    },
    /// A sync operation was skipped.
    SyncSkipped {
        repo_name: String,
        reason: String,
        index: usize,
        total: usize,
    },
}

/// A cloneable progress adapter that forwards all events to a host-provided sink.
#[derive(Clone)]
pub struct ProgressReporter {
    sink: Arc<dyn Fn(ProgressEvent) + Send + Sync>,
}

impl ProgressReporter {
    /// Creates a reporter that forwards each progress event to `sink`.
    pub fn new(sink: impl Fn(ProgressEvent) + Send + Sync + 'static) -> Self {
        Self {
            sink: Arc::new(sink),
        }
    }

    fn emit(&self, event: ProgressEvent) {
        (self.sink)(event);
    }
}

impl DiscoveryProgress for ProgressReporter {
    fn on_orgs_discovered(&self, count: usize) {
        self.emit(ProgressEvent::DiscoveryOrgsDiscovered { count });
    }

    fn on_org_started(&self, org_name: &str) {
        self.emit(ProgressEvent::DiscoveryOrgStarted {
            org_name: org_name.to_string(),
        });
    }

    fn on_org_complete(&self, org_name: &str, repo_count: usize) {
        self.emit(ProgressEvent::DiscoveryOrgComplete {
            org_name: org_name.to_string(),
            repo_count,
        });
    }

    fn on_personal_repos_started(&self) {
        self.emit(ProgressEvent::DiscoveryPersonalReposStarted);
    }

    fn on_personal_repos_complete(&self, count: usize) {
        self.emit(ProgressEvent::DiscoveryPersonalReposComplete { count });
    }

    fn on_error(&self, message: &str) {
        self.emit(ProgressEvent::DiscoveryError {
            message: message.to_string(),
        });
    }
}

impl CloneProgress for ProgressReporter {
    fn on_start(&self, repo: &OwnedRepo, index: usize, total: usize) {
        self.emit(ProgressEvent::CloneStarted {
            repo_name: repo.full_name().to_string(),
            index,
            total,
        });
    }

    fn on_complete(&self, repo: &OwnedRepo, index: usize, total: usize) {
        self.emit(ProgressEvent::CloneCompleted {
            repo_name: repo.full_name().to_string(),
            index,
            total,
        });
    }

    fn on_error(&self, repo: &OwnedRepo, error: &str, index: usize, total: usize) {
        self.emit(ProgressEvent::CloneFailed {
            repo_name: repo.full_name().to_string(),
            error: error.to_string(),
            index,
            total,
        });
    }

    fn on_skip(&self, repo: &OwnedRepo, reason: &str, index: usize, total: usize) {
        self.emit(ProgressEvent::CloneSkipped {
            repo_name: repo.full_name().to_string(),
            reason: reason.to_string(),
            index,
            total,
        });
    }
}

impl SyncProgress for ProgressReporter {
    fn on_start(&self, repo: &OwnedRepo, path: &Path, index: usize, total: usize) {
        self.emit(ProgressEvent::SyncStarted {
            repo_name: repo.full_name().to_string(),
            path: path.display().to_string(),
            index,
            total,
        });
    }

    fn on_fetch_complete(
        &self,
        repo: &OwnedRepo,
        result: &FetchResult,
        index: usize,
        total: usize,
    ) {
        self.emit(ProgressEvent::SyncFetched {
            repo_name: repo.full_name().to_string(),
            updated: result.updated,
            new_commits: result.new_commits,
            index,
            total,
        });
    }

    fn on_pull_complete(&self, repo: &OwnedRepo, result: &PullResult, index: usize, total: usize) {
        self.emit(ProgressEvent::SyncPulled {
            repo_name: repo.full_name().to_string(),
            success: result.success,
            updated: result.updated,
            fast_forward: result.fast_forward,
            error: result.error.clone(),
            index,
            total,
        });
    }

    fn on_error(&self, repo: &OwnedRepo, error: &str, index: usize, total: usize) {
        self.emit(ProgressEvent::SyncFailed {
            repo_name: repo.full_name().to_string(),
            error: error.to_string(),
            index,
            total,
        });
    }

    fn on_skip(&self, repo: &OwnedRepo, reason: &str, index: usize, total: usize) {
        self.emit(ProgressEvent::SyncSkipped {
            repo_name: repo.full_name().to_string(),
            reason: reason.to_string(),
            index,
            total,
        });
    }
}

#[cfg(test)]
#[path = "progress_tests.rs"]
mod tests;
