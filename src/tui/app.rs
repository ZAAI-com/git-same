//! TUI application state (the "Model" in Elm architecture).

use crate::config::{Config, WorkspaceConfig};
use crate::setup::state::{self, SetupState};
use crate::types::{OpSummary, OwnedRepo};
use ratatui::widgets::TableState;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Instant;

/// Which screen is active.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    InitCheck,
    SetupWizard,
    WorkspaceSelector,
    Dashboard,
    Progress,
    Settings,
}

/// Which operation is running or was last selected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operation {
    Sync,
    Status,
}

impl std::fmt::Display for Operation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Operation::Sync => write!(f, "Sync"),
            Operation::Status => write!(f, "Status"),
        }
    }
}

/// State of an ongoing async operation.
#[derive(Debug, Clone)]
pub enum OperationState {
    Idle,
    Discovering {
        message: String,
    },
    Running {
        operation: Operation,
        total: usize,
        completed: usize,
        failed: usize,
        skipped: usize,
        current_repo: String,
        /// Repos that had new commits (updated or cloned).
        with_updates: usize,
        /// New repos cloned so far.
        cloned: usize,
        /// Existing repos synced so far.
        synced: usize,
        /// Planned clone count (for phase indicator).
        to_clone: usize,
        /// Planned sync count (for phase indicator).
        to_sync: usize,
        /// Aggregate new commits fetched.
        total_new_commits: u32,
        /// When the operation started (for elapsed/ETA).
        started_at: Instant,
        /// Repos currently being processed (for worker slots).
        active_repos: Vec<String>,
        /// Throughput samples (repos completed per second window).
        throughput_samples: Vec<u64>,
        /// Completed count at last throughput sample.
        last_sample_completed: usize,
    },
    Finished {
        operation: Operation,
        summary: OpSummary,
        /// Repos that had new commits.
        with_updates: usize,
        /// New repos cloned.
        cloned: usize,
        /// Existing repos synced.
        synced: usize,
        /// Aggregate new commits fetched.
        total_new_commits: u32,
        /// Wall-clock duration in seconds.
        duration_secs: f64,
    },
}

/// A structured log entry from a sync operation.
#[derive(Debug, Clone)]
pub struct SyncLogEntry {
    pub repo_name: String,
    pub status: SyncLogStatus,
    pub message: String,
    pub had_updates: bool,
    pub is_clone: bool,
    pub new_commits: Option<u32>,
    pub path: Option<PathBuf>,
}

/// Status classification for a sync log entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncLogStatus {
    Success,
    Updated,
    Cloned,
    Failed,
    Skipped,
}

/// Filter for post-sync log view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogFilter {
    All,
    Updated,
    Failed,
    Skipped,
    Changelog,
}

/// A summary entry for sync history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncHistoryEntry {
    pub timestamp: String,
    pub duration_secs: f64,
    pub success: usize,
    pub failed: usize,
    pub skipped: usize,
    pub with_updates: usize,
    pub cloned: usize,
    pub total_new_commits: u32,
}

/// A local repo with its computed status.
#[derive(Debug, Clone)]
pub struct RepoEntry {
    pub owner: String,
    pub name: String,
    pub full_name: String,
    pub path: PathBuf,
    pub branch: Option<String>,
    pub is_uncommitted: bool,
    pub ahead: usize,
    pub behind: usize,
    pub staged_count: usize,
    pub unstaged_count: usize,
    pub untracked_count: usize,
}

/// A requirement check result for the init check screen.
#[derive(Debug, Clone)]
pub struct CheckEntry {
    pub name: String,
    pub passed: bool,
    pub message: String,
    pub critical: bool,
}

/// The application model (all TUI state).
pub struct App {
    /// Whether the user has requested quit.
    pub should_quit: bool,

    /// Whether the first 'q' has been pressed (waiting for second 'q' to confirm quit).
    pub quit_pending: bool,

    /// Active screen.
    pub screen: Screen,

    /// Screen history for back navigation.
    pub screen_stack: Vec<Screen>,

    /// Loaded configuration.
    pub config: Config,

    /// Available workspaces.
    pub workspaces: Vec<WorkspaceConfig>,

    /// Active workspace (selected or auto-selected).
    pub active_workspace: Option<WorkspaceConfig>,

    /// Selected index in workspace selector.
    pub workspace_index: usize,

    /// Base path for repos (derived from active workspace).
    pub base_path: Option<PathBuf>,

    /// Discovered repos grouped by org.
    pub repos_by_org: HashMap<String, Vec<OwnedRepo>>,

    /// All discovered repos (flat list).
    pub all_repos: Vec<OwnedRepo>,

    /// Org names (sorted).
    pub orgs: Vec<String>,

    /// Local repo entries with status.
    pub local_repos: Vec<RepoEntry>,

    /// Current async operation state.
    pub operation_state: OperationState,

    /// Operation log lines (last N events).
    pub log_lines: Vec<String>,

    // -- Selection state --
    /// Selected repo index in current view.
    pub repo_index: usize,

    /// Scroll offset for tables.
    pub scroll_offset: usize,

    /// Filter/search text.
    pub filter_text: String,

    /// Whether filter input is active.
    pub filter_active: bool,

    /// Whether dry-run is toggled in command picker.
    pub dry_run: bool,

    /// Error message to display (clears on next keypress).
    pub error_message: Option<String>,

    /// Requirement check results (populated on InitCheck screen).
    pub check_results: Vec<CheckEntry>,

    /// Whether checks are still running.
    pub checks_loading: bool,

    /// Whether to use pull mode for sync (vs fetch).
    pub sync_pull: bool,

    /// Setup wizard state (active when on SetupWizard screen).
    pub setup_state: Option<SetupState>,

    /// Whether the config file was successfully created by init.
    pub config_created: bool,

    /// Path where config was written (for display).
    pub config_path_display: Option<String>,

    /// Whether status scan is in progress.
    pub status_loading: bool,

    /// When the last status scan completed (for auto-refresh cooldown).
    pub last_status_scan: Option<std::time::Instant>,

    /// Selected stat box index on dashboard (0-5) for ←/→ navigation.
    pub stat_index: usize,

    /// Table state for dashboard tab content (tracks selection + scroll offset).
    pub dashboard_table_state: TableState,

    /// Selected category index in settings screen (0 = Requirements, 1 = Options, 2+ = Workspaces).
    pub settings_index: usize,

    /// Whether the config TOML section is expanded in workspace detail.
    pub settings_config_expanded: bool,

    /// Tick counter for driving animations on the Progress screen.
    pub tick_count: u64,

    /// Structured sync log entries (enriched data).
    pub sync_log_entries: Vec<SyncLogEntry>,

    /// Active log filter for post-sync view.
    pub log_filter: LogFilter,

    /// Sync history (last N summaries for comparison).
    pub sync_history: Vec<SyncHistoryEntry>,

    /// Whether sync history overlay is visible.
    pub show_sync_history: bool,

    /// Expanded repo in post-sync view (for commit deep dive).
    pub expanded_repo: Option<String>,

    /// Commit log for expanded repo.
    pub repo_commits: Vec<String>,

    /// Selected index in the post-sync filterable log.
    pub sync_log_index: usize,
}

impl App {
    /// Create a new App with the given config and workspaces.
    pub fn new(config: Config, workspaces: Vec<WorkspaceConfig>) -> Self {
        let (screen, active_workspace, base_path) = match workspaces.len() {
            0 => (Screen::SetupWizard, None, None),
            1 => {
                let ws = workspaces[0].clone();
                let bp = Some(ws.expanded_base_path());
                (Screen::Dashboard, Some(ws), bp)
            }
            _ => {
                // Check for default workspace
                if let Some(ref default_name) = config.default_workspace {
                    if let Some(ws) = workspaces.iter().find(|w| w.name == *default_name) {
                        let bp = Some(ws.expanded_base_path());
                        (Screen::Dashboard, Some(ws.clone()), bp)
                    } else {
                        (Screen::WorkspaceSelector, None, None)
                    }
                } else {
                    (Screen::WorkspaceSelector, None, None)
                }
            }
        };

        let sync_history = active_workspace
            .as_ref()
            .and_then(|ws| {
                crate::cache::SyncHistoryManager::for_workspace(&ws.name)
                    .and_then(|m| m.load())
                    .ok()
            })
            .unwrap_or_default();

        Self {
            should_quit: false,
            quit_pending: false,
            screen,
            screen_stack: Vec::new(),
            config,
            workspaces,
            active_workspace,
            workspace_index: 0,
            base_path,
            repos_by_org: HashMap::new(),
            all_repos: Vec::new(),
            orgs: Vec::new(),
            local_repos: Vec::new(),
            operation_state: OperationState::Idle,
            log_lines: Vec::new(),
            repo_index: 0,
            scroll_offset: 0,
            filter_text: String::new(),
            filter_active: false,
            dry_run: false,
            error_message: None,
            check_results: Vec::new(),
            checks_loading: false,
            sync_pull: false,
            setup_state: if screen == Screen::SetupWizard {
                let default_path = std::env::current_dir()
                    .map(|p| state::tilde_collapse(&p.to_string_lossy()))
                    .unwrap_or_else(|_| "~/Git-Same/GitHub".to_string());
                Some(SetupState::new(&default_path))
            } else {
                None
            },
            config_created: false,
            config_path_display: None,
            status_loading: false,
            last_status_scan: None,
            stat_index: 0,
            dashboard_table_state: TableState::default().with_selected(0),
            settings_index: 0,
            settings_config_expanded: false,
            tick_count: 0,
            sync_log_entries: Vec::new(),
            log_filter: LogFilter::All,
            sync_history,
            show_sync_history: false,
            expanded_repo: None,
            repo_commits: Vec::new(),
            sync_log_index: 0,
        }
    }

    /// Select a workspace and navigate to dashboard.
    pub fn select_workspace(&mut self, index: usize) {
        if let Some(ws) = self.workspaces.get(index).cloned() {
            self.base_path = Some(ws.expanded_base_path());
            // Load sync history for this workspace
            self.sync_history = crate::cache::SyncHistoryManager::for_workspace(&ws.name)
                .and_then(|m| m.load())
                .unwrap_or_default();
            self.active_workspace = Some(ws);
            // Reset discovered data when switching workspace
            self.repos_by_org.clear();
            self.all_repos.clear();
            self.orgs.clear();
            self.local_repos.clear();
            self.last_status_scan = None;
        }
    }

    /// Navigate to a new screen, pushing current onto the stack.
    pub fn navigate_to(&mut self, screen: Screen) {
        self.screen_stack.push(self.screen);
        self.screen = screen;
        self.repo_index = 0;
        self.scroll_offset = 0;
    }

    /// Go back to previous screen.
    pub fn go_back(&mut self) {
        if let Some(prev) = self.screen_stack.pop() {
            self.screen = prev;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_no_workspaces_shows_setup_wizard() {
        let app = App::new(Config::default(), vec![]);
        assert_eq!(app.screen, Screen::SetupWizard);
        assert!(app.setup_state.is_some());
        assert!(app.active_workspace.is_none());
        assert!(app.base_path.is_none());
    }

    #[test]
    fn test_new_single_workspace_auto_selects() {
        let ws = WorkspaceConfig::new("test", "/tmp/test");
        let app = App::new(Config::default(), vec![ws]);
        assert_eq!(app.screen, Screen::Dashboard);
        assert!(app.active_workspace.is_some());
        assert_eq!(app.active_workspace.unwrap().name, "test");
        assert!(app.base_path.is_some());
    }

    #[test]
    fn test_new_multiple_no_default_shows_selector() {
        let ws1 = WorkspaceConfig::new("ws1", "/tmp/ws1");
        let ws2 = WorkspaceConfig::new("ws2", "/tmp/ws2");
        let app = App::new(Config::default(), vec![ws1, ws2]);
        assert_eq!(app.screen, Screen::WorkspaceSelector);
        assert!(app.active_workspace.is_none());
    }

    #[test]
    fn test_new_multiple_with_valid_default_auto_selects() {
        let ws1 = WorkspaceConfig::new("ws1", "/tmp/ws1");
        let ws2 = WorkspaceConfig::new("ws2", "/tmp/ws2");
        let mut config = Config::default();
        config.default_workspace = Some("ws2".to_string());
        let app = App::new(config, vec![ws1, ws2]);
        assert_eq!(app.screen, Screen::Dashboard);
        assert_eq!(app.active_workspace.unwrap().name, "ws2");
    }

    #[test]
    fn test_new_multiple_with_invalid_default_shows_selector() {
        let ws1 = WorkspaceConfig::new("ws1", "/tmp/ws1");
        let ws2 = WorkspaceConfig::new("ws2", "/tmp/ws2");
        let mut config = Config::default();
        config.default_workspace = Some("nonexistent".to_string());
        let app = App::new(config, vec![ws1, ws2]);
        assert_eq!(app.screen, Screen::WorkspaceSelector);
        assert!(app.active_workspace.is_none());
    }
}
