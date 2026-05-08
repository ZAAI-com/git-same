//! Background classification of org folder owners (User vs Organization).
//!
//! Spawned once at monitor startup. Walks every configured workspace,
//! resolves any owner names that aren't yet classified via the GitHub API,
//! and persists results in `OwnerTypeCache`. Subsequent scans pick up the
//! new classifications automatically.

use crate::api::OwnerTypeCache;
use crate::config::Config;
use crate::types::OwnerType;
use std::collections::BTreeSet;
use tracing::{debug, info, warn};

/// Spawn the classifier on the current tokio runtime. Returns immediately.
pub fn spawn_owner_classifier(config: Config, cache: OwnerTypeCache) {
    tokio::spawn(async move {
        let names = collect_owner_names(&config);
        let missing = cache.missing(names.iter().map(|s| s.as_str()));
        if missing.is_empty() {
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
        let ws_provider = crate::config::WorkspaceProvider::default();
        let provider = match crate::provider::create_provider(&ws_provider, &token) {
            Ok(p) => p,
            Err(e) => {
                warn!(error = %e, "Owner classification skipped: provider init failed");
                return;
            }
        };

        info!(
            count = missing.len(),
            "Classifying owner types via GitHub API"
        );
        for name in &missing {
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
                    let _ = cache.set(name, OwnerType::Unknown);
                }
            }
        }
        info!("Owner classification complete");
    });
}

/// Collect every unique top-level folder name (orgs + users) across all
/// configured workspaces.
fn collect_owner_names(config: &Config) -> Vec<String> {
    let mut names: BTreeSet<String> = BTreeSet::new();

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

    names.into_iter().collect()
}
