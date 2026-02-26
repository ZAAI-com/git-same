//! Repository path templating.

use crate::types::OwnedRepo;
use std::path::{Path, PathBuf};

/// Canonical renderer for workspace repository paths.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoPathTemplate {
    template: String,
}

impl RepoPathTemplate {
    /// Create a new path template.
    pub fn new(template: impl Into<String>) -> Self {
        Self {
            template: template.into(),
        }
    }

    /// Returns the underlying template string.
    pub fn as_str(&self) -> &str {
        &self.template
    }

    /// Render a repository path from template placeholders.
    pub fn render(&self, base_path: &Path, provider: &str, owner: &str, repo: &str) -> PathBuf {
        let rendered = self
            .template
            .replace("{provider}", provider)
            .replace("{org}", owner)
            .replace("{repo}", repo);

        base_path.join(rendered)
    }

    /// Render a repository path from an owned repository object.
    pub fn render_owned_repo(&self, base_path: &Path, repo: &OwnedRepo, provider: &str) -> PathBuf {
        self.render(base_path, provider, &repo.owner, &repo.repo.name)
    }

    /// Render from a full name (`org/repo`) when available.
    pub fn render_full_name(
        &self,
        base_path: &Path,
        provider: &str,
        full_name: &str,
    ) -> Option<PathBuf> {
        let (owner, repo) = full_name.split_once('/')?;
        Some(self.render(base_path, provider, owner, repo))
    }

    /// Expected scan depth for local repository traversal.
    pub fn scan_depth(&self) -> usize {
        if self.template.contains("{provider}") {
            3
        } else {
            2
        }
    }
}

impl Default for RepoPathTemplate {
    fn default() -> Self {
        Self::new("{org}/{repo}")
    }
}

#[cfg(test)]
#[path = "repo_path_template_tests.rs"]
mod tests;
