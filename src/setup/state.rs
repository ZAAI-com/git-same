//! Setup wizard state (the "Model" in Elm architecture).

use crate::config::WorkspaceProvider;
use crate::types::ProviderKind;

/// Which step of the wizard is active.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetupStep {
    /// Step 0: Welcome screen (first-time only).
    Welcome,
    /// Step 1: Select a provider.
    SelectProvider,
    /// Step 2: Authenticate and detect username.
    Authenticate,
    /// Step 3: Discover and select organizations.
    SelectOrgs,
    /// Step 4: Enter the base path.
    SelectPath,
    /// Step 5: Review and save.
    Confirm,
    /// Step 6: Success / completion screen.
    Complete,
}

/// An organization entry in the org selector.
#[derive(Debug, Clone)]
pub struct OrgEntry {
    pub name: String,
    pub repo_count: usize,
    pub selected: bool,
}

/// The outcome of the setup wizard.
#[derive(Debug, Clone)]
pub enum SetupOutcome {
    /// User completed the wizard.
    Completed,
    /// User cancelled.
    Cancelled,
}

/// Represents one of the provider choices shown in step 1.
#[derive(Debug, Clone)]
pub struct ProviderChoice {
    pub kind: ProviderKind,
    pub label: String,
    pub available: bool,
}

/// A suggested directory path for the path selector.
#[derive(Debug, Clone)]
pub struct PathSuggestion {
    pub path: String,
    pub label: String,
}

/// A selectable directory entry in the inline path navigator.
#[derive(Debug, Clone)]
pub struct PathBrowseEntry {
    pub label: String,
    pub path: String,
}

/// The wizard state (model).
pub struct SetupState {
    /// Current wizard step.
    pub step: SetupStep,
    /// Whether to quit the wizard.
    pub should_quit: bool,
    /// Outcome when done.
    pub outcome: Option<SetupOutcome>,

    // Step 1: Provider selection
    pub provider_choices: Vec<ProviderChoice>,
    pub provider_index: usize,

    // Step 2: Authentication
    pub auth_status: AuthStatus,
    pub username: Option<String>,
    pub auth_token: Option<String>,

    // Step 3: Org selection
    pub orgs: Vec<OrgEntry>,
    pub org_index: usize,
    pub org_loading: bool,
    pub org_error: Option<String>,

    // Step 4: Path
    pub base_path: String,
    pub path_cursor: usize,
    pub path_suggestions_mode: bool,
    pub path_suggestions: Vec<PathSuggestion>,
    pub path_suggestion_index: usize,
    pub path_completions: Vec<String>,
    pub path_completion_index: usize,
    pub path_browse_mode: bool,
    pub path_browse_current_dir: String,
    pub path_browse_entries: Vec<PathBrowseEntry>,
    pub path_browse_index: usize,
    pub path_browse_show_hidden: bool,
    pub path_browse_error: Option<String>,
    pub path_browse_info: Option<String>,

    // Step 5: Confirm
    pub workspace_name: String,
    pub name_editing: bool,

    // General
    pub error_message: Option<String>,

    // Animation / UX
    /// Tick counter for spinner and animation effects.
    pub tick_count: u64,
    /// Whether this is the first workspace setup (controls Welcome screen).
    pub is_first_setup: bool,
}

/// Authentication status during step 2.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthStatus {
    /// Haven't checked yet.
    Pending,
    /// Currently checking.
    Checking,
    /// Authenticated successfully.
    Success,
    /// Authentication failed.
    Failed(String),
}

/// Collapse an absolute path's home directory prefix into `~`.
pub fn tilde_collapse(path: &str) -> String {
    if let Ok(home) = std::env::var("HOME") {
        if path.starts_with(&home) {
            return format!("~{}", &path[home.len()..]);
        }
    }
    path.to_string()
}

impl SetupState {
    /// Create initial wizard state.
    ///
    /// If `is_first_setup` is true, the wizard starts with a Welcome screen.
    pub fn new(default_base_path: &str) -> Self {
        Self::with_first_setup(default_base_path, false)
    }

    /// Create wizard state, optionally starting with the Welcome screen.
    pub fn with_first_setup(default_base_path: &str, is_first_setup: bool) -> Self {
        let provider_choices = vec![
            ProviderChoice {
                kind: ProviderKind::GitHub,
                label: "GitHub".to_string(),
                available: true,
            },
            ProviderChoice {
                kind: ProviderKind::GitHubEnterprise,
                label: "GitHub Enterprise (coming soon)".to_string(),
                available: false,
            },
            ProviderChoice {
                kind: ProviderKind::GitLab,
                label: "GitLab.com (coming soon)".to_string(),
                available: false,
            },
            ProviderChoice {
                kind: ProviderKind::GitLabSelfManaged,
                label: "GitLab Self-Managed (coming soon)".to_string(),
                available: false,
            },
            ProviderChoice {
                kind: ProviderKind::Codeberg,
                label: "Codeberg.org (coming soon)".to_string(),
                available: false,
            },
            ProviderChoice {
                kind: ProviderKind::Bitbucket,
                label: "Bitbucket.org (coming soon)".to_string(),
                available: false,
            },
        ];

        let base_path = default_base_path.to_string();
        let path_cursor = base_path.len();

        let step = if is_first_setup {
            SetupStep::Welcome
        } else {
            SetupStep::SelectProvider
        };

        Self {
            step,
            should_quit: false,
            outcome: None,
            provider_choices,
            provider_index: 0,
            auth_status: AuthStatus::Pending,
            username: None,
            auth_token: None,
            base_path,
            path_cursor,
            path_suggestions_mode: true,
            path_suggestions: Vec::new(),
            path_suggestion_index: 0,
            path_completions: Vec::new(),
            path_completion_index: 0,
            path_browse_mode: false,
            path_browse_current_dir: String::new(),
            path_browse_entries: Vec::new(),
            path_browse_index: 0,
            path_browse_show_hidden: false,
            path_browse_error: None,
            path_browse_info: None,
            orgs: Vec::new(),
            org_index: 0,
            org_loading: false,
            org_error: None,
            workspace_name: String::new(),
            name_editing: false,
            error_message: None,
            tick_count: 0,
            is_first_setup,
        }
    }

    /// Get the selected provider kind.
    pub fn selected_provider(&self) -> ProviderKind {
        self.provider_choices[self.provider_index].kind
    }

    /// Build the WorkspaceProvider from current state.
    pub fn build_workspace_provider(&self) -> WorkspaceProvider {
        let kind = self.selected_provider();
        WorkspaceProvider {
            kind,
            api_url: None,
            ..WorkspaceProvider::default()
        }
    }

    /// Get selected org names.
    pub fn selected_orgs(&self) -> Vec<String> {
        self.orgs
            .iter()
            .filter(|o| o.selected)
            .map(|o| o.name.clone())
            .collect()
    }

    /// Populate the path suggestions list for the SelectPath step.
    pub fn populate_path_suggestions(&mut self) {
        let mut suggestions = Vec::new();

        // 1. Current path (always first — this is the default)
        suggestions.push(PathSuggestion {
            path: self.base_path.clone(),
            label: "current directory".to_string(),
        });

        // 2. Common developer directories (only if they exist and differ)
        for candidate in &[
            "~/Git-Same/GitHub",
            "~/Developer",
            "~/Projects",
            "~/repos",
            "~/code",
        ] {
            let expanded = shellexpand::tilde(candidate);
            let path = std::path::Path::new(expanded.as_ref());
            if path.is_dir() && !suggestions.iter().any(|s| s.path == *candidate) {
                suggestions.push(PathSuggestion {
                    path: candidate.to_string(),
                    label: String::new(),
                });
            }
        }

        // 3. Home directory (always last)
        if !suggestions.iter().any(|s| s.path == "~") {
            suggestions.push(PathSuggestion {
                path: "~".to_string(),
                label: "home".to_string(),
            });
        }

        self.path_suggestions = suggestions;
        self.path_suggestion_index = 0;
        self.path_suggestions_mode = true;
        self.path_browse_mode = false;
        self.path_browse_current_dir.clear();
        self.path_browse_entries.clear();
        self.path_browse_index = 0;
        self.path_browse_show_hidden = false;
        self.path_browse_error = None;
        self.path_browse_info = None;
    }

    /// The 1-based step number for display (Welcome is not counted).
    pub fn step_number(&self) -> usize {
        match self.step {
            SetupStep::Welcome => 0,
            SetupStep::SelectProvider => 1,
            SetupStep::Authenticate => 2,
            SetupStep::SelectOrgs => 3,
            SetupStep::SelectPath => 4,
            SetupStep::Confirm => 5,
            SetupStep::Complete => 5,
        }
    }

    /// Total number of numbered steps (excluding Welcome and Complete).
    pub const TOTAL_STEPS: usize = 5;

    /// Move to the next step.
    pub fn next_step(&mut self) {
        self.error_message = None;
        self.step = match self.step {
            SetupStep::Welcome => SetupStep::SelectProvider,
            SetupStep::SelectProvider => SetupStep::Authenticate,
            SetupStep::Authenticate => {
                self.org_loading = true;
                self.orgs.clear();
                self.org_index = 0;
                self.org_error = None;
                SetupStep::SelectOrgs
            }
            SetupStep::SelectOrgs => {
                self.populate_path_suggestions();
                SetupStep::SelectPath
            }
            SetupStep::SelectPath => {
                // Derive workspace name from base_path + provider
                let path = std::path::Path::new(&self.base_path);
                let base =
                    crate::config::WorkspaceManager::name_from_path(path, self.selected_provider());
                self.workspace_name =
                    crate::config::WorkspaceManager::unique_name(&base).unwrap_or(base);
                SetupStep::Confirm
            }
            SetupStep::Confirm => SetupStep::Complete,
            SetupStep::Complete => {
                self.outcome = Some(SetupOutcome::Completed);
                self.should_quit = true;
                SetupStep::Complete
            }
        };
    }

    /// Move to the previous step.
    pub fn prev_step(&mut self) {
        self.error_message = None;
        self.step = match self.step {
            SetupStep::Welcome => {
                self.outcome = Some(SetupOutcome::Cancelled);
                self.should_quit = true;
                SetupStep::Welcome
            }
            SetupStep::SelectProvider => {
                self.outcome = Some(SetupOutcome::Cancelled);
                self.should_quit = true;
                SetupStep::SelectProvider
            }
            SetupStep::Authenticate => SetupStep::SelectProvider,
            SetupStep::SelectOrgs => SetupStep::Authenticate,
            SetupStep::SelectPath => SetupStep::SelectOrgs,
            SetupStep::Confirm => SetupStep::SelectPath,
            SetupStep::Complete => SetupStep::Confirm,
        };
    }
}

#[cfg(test)]
#[path = "state_tests.rs"]
mod tests;
