//! TUI application state (the "Model" in Elm architecture).

use crate::config::Config;
use crate::types::{OpSummary, OwnedRepo};
use std::collections::HashMap;
use std::path::PathBuf;

/// Which screen is active.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Dashboard,
    CommandPicker,
    OrgBrowser,
    Progress,
    RepoStatus,
}

/// Which operation is running or was last selected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operation {
    Clone,
    Fetch,
    Pull,
    Status,
}

impl std::fmt::Display for Operation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Operation::Clone => write!(f, "Clone"),
            Operation::Fetch => write!(f, "Fetch"),
            Operation::Pull => write!(f, "Pull"),
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

    /// Base path for repos (from config).
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
}

impl App {
    /// Create a new App with the given config.
    pub fn new(config: Config) -> Self {
        let base_path = if config.base_path.is_empty() {
            None
        } else {
            let expanded = shellexpand::tilde(&config.base_path);
            Some(PathBuf::from(expanded.as_ref()))
        };
        Self {
            should_quit: false,
            screen: Screen::Dashboard,
            screen_stack: Vec::new(),
            config,
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
