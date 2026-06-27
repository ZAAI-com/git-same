//! Background classification of org folder owners (User vs Organization).
//!
//! Spawned once at monitor startup. Walks every configured workspace,
//! resolves any owner names that aren't yet classified via the GitHub API,
//! and persists results in `OwnerTypeCache`. Subsequent scans pick up the
//! new classifications automatically.

use crate::api::OwnerTypeCache;
use crate::config::{Config, WorkspaceProvider};
use crate::types::OwnerType;
use std::collections::{BTreeMap, BTreeSet};
use tracing::{debug, info, warn};

/// Spawn the classifier on the current tokio runtime. Returns immediately.
pub fn spawn_owner_classifier(config: Config, cache: OwnerTypeCache) {
    tokio::spawn(async move {
        // Owners are grouped by provider endpoint: each GitHub instance
        // (github.com vs a GitHub Enterprise host) gets its own client and
        // classification pass, so a GHE workspace is never queried against
        // github.com.
        let groups = collect_owner_names_by_provider(&config);
        let pending: Vec<(WorkspaceProvider, Vec<String>)> = groups
            .into_iter()
            .filter_map(|(provider, names)| {
                let missing = cache.missing(names.iter().map(|s| s.as_str()));
                if missing.is_empty() {
                    None
                } else {
                    Some((provider, missing))
                }
            })
            .collect();

        if pending.is_empty() {
            debug!("Owner type cache already populated, skipping classification");
            return;
        }

        let token = match crate::auth::gh_cli::get_token() {
            Ok(t) => t,
            Err(e) => {
                warn!(error = %e, "Owner classification skipped: gh auth token unavailable");
                return;
            }
        };

        let total: usize = pending.iter().map(|(_, names)| names.len()).sum();
        info!(count = total, "Classifying owner types via GitHub API");

        for (ws_provider, names) in pending {
            let provider = match crate::provider::create_provider(&ws_provider, &token) {
                Ok(p) => p,
                Err(e) => {
                    warn!(
                        provider = %ws_provider.display_name(),
                        api_url = %ws_provider.effective_api_url(),
                        error = %e,
                        "Owner classification skipped for provider: init failed"
                    );
                    continue;
                }
            };
            for name in &names {
                match provider.get_owner_type(name).await {
                    Ok(ot) => {
                        if let Err(e) = cache.set(name, ot) {
                            warn!(name = %name, error = %e, "Failed to persist owner type");
                        } else {
                            debug!(name = %name, owner_type = ?ot, "Classified owner");
                        }
                    }
                    Err(e) => {
                        debug!(name = %name, error = %e, "Owner classification failed, leaving unknown");
                        if let Err(persist_err) = cache.set(name, OwnerType::Unknown) {
                            warn!(name = %name, error = %persist_err, "Failed to persist owner type");
                        }
                    }
                }
            }
        }
        info!("Owner classification complete");
    });
}

/// Collect every unique top-level folder name (orgs + users), grouped by the
/// provider endpoint of the workspace it came from.
///
/// Workspaces that point at the same GitHub instance (same effective API URL)
/// are merged into one group and classified with a single shared client;
/// distinct endpoints (e.g. a GitHub Enterprise host) each get their own group.
fn collect_owner_names_by_provider(config: &Config) -> Vec<(WorkspaceProvider, Vec<String>)> {
    // Keyed by effective API URL so multiple workspaces on the same instance
    // collapse together. The first provider seen for a key wins (they are
    // equivalent for classification purposes).
    let mut groups: BTreeMap<String, (WorkspaceProvider, BTreeSet<String>)> = BTreeMap::new();

    for ws_path in &config.workspaces {
        let expanded = shellexpand::tilde(ws_path).to_string();
        let root = std::path::PathBuf::from(&expanded);
        if !root.exists() {
            continue;
        }
        let ws_config = match crate::config::WorkspaceStore::load(&root) {
            Ok(ws) => ws,
            Err(_) => continue,
        };

        let key = ws_config.provider.effective_api_url();
        let (_, names) = groups
            .entry(key)
            .or_insert_with(|| (ws_config.provider.clone(), BTreeSet::new()));

        let base_path = ws_config.expanded_base_path();
        if !ws_config.orgs.is_empty() {
            names.extend(ws_config.orgs.iter().cloned());
        } else if let Ok(entries) = std::fs::read_dir(&base_path) {
            for e in entries.flatten() {
                if let Some(n) = e.file_name().to_str() {
                    if !n.starts_with('.') && e.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                        names.insert(n.to_string());
                    }
                }
            }
        }
        if !ws_config.username.is_empty() {
            names.insert(ws_config.username.clone());
        }
    }

    groups
        .into_values()
        .map(|(provider, names)| (provider, names.into_iter().collect()))
        .collect()
}

#[cfg(test)]
#[path = "owner_classifier_tests.rs"]
mod tests;
