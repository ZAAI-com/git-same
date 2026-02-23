//! TUI application state (the "Model" in Elm architecture).

use crate::config::{Config, WorkspaceConfig};
use crate::types::{OpSummary, OwnedRepo};
use std::collections::HashMap;
use std::path::PathBuf;

/// Which screen is active.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    InitCheck,
    WorkspaceSelector,
    Dashboard,
    CommandPicker,
    OrgBrowser,
    Progress,
    RepoStatus,
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
    },
    Finished {
        operation: Operation,
        summary: OpSummary,
    },
}

/// A local repo with its computed status.
#[derive(Debug, Clone)]
pub struct RepoEntry {
    pub owner: String,
    pub name: String,
    pub full_name: String,
    pub path: PathBuf,
    pub branch: Option<String>,
    pub is_dirty: bool,
    pub ahead: usize,
    pub behind: usize,
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
    /// Selected index in command picker.
    pub picker_index: usize,

    /// Selected org index in org browser.
    pub org_index: usize,

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

    /// Whether dirty-only filter is active in repo status.
    pub filter_dirty: bool,

    /// Whether behind-only filter is active in repo status.
    pub filter_behind: bool,

    /// Requirement check results (populated on InitCheck screen).
    pub check_results: Vec<CheckEntry>,

    /// Whether checks are still running.
    pub checks_loading: bool,

    /// Whether to use pull mode for sync (vs fetch).
    pub sync_pull: bool,
}

impl App {
    /// Create a new App with the given config and workspaces.
    pub fn new(config: Config, workspaces: Vec<WorkspaceConfig>) -> Self {
        let (screen, active_workspace, base_path) = match workspaces.len() {
            0 => (Screen::InitCheck, None, None),
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

        Self {
            should_quit: false,
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
            picker_index: 0,
            org_index: 0,
            repo_index: 0,
            scroll_offset: 0,
            filter_text: String::new(),
            filter_active: false,
            dry_run: false,
            error_message: None,
            filter_dirty: false,
            filter_behind: false,
            check_results: Vec::new(),
            checks_loading: false,
            sync_pull: false,
        }
    }

    /// Select a workspace and navigate to dashboard.
    pub fn select_workspace(&mut self, index: usize) {
        if let Some(ws) = self.workspaces.get(index).cloned() {
            self.base_path = Some(ws.expanded_base_path());
            self.active_workspace = Some(ws);
            // Reset discovered data when switching workspace
            self.repos_by_org.clear();
            self.all_repos.clear();
            self.orgs.clear();
            self.local_repos.clear();
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
    fn test_new_no_workspaces_shows_init_check() {
        let app = App::new(Config::default(), vec![]);
        assert_eq!(app.screen, Screen::InitCheck);
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
