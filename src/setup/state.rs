//! Setup wizard state (the "Model" in Elm architecture).

use crate::config::{AuthMethod, WorkspaceProvider};
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
    /// Step 3: Enter the base path.
    SelectPath,
    /// Step 4: Discover and select organizations.
    SelectOrgs,
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

    // Step 3: Path
    pub base_path: String,
    pub path_cursor: usize,
    pub path_suggestions_mode: bool,
    pub path_suggestions: Vec<PathSuggestion>,
    pub path_suggestion_index: usize,
    pub path_completions: Vec<String>,
    pub path_completion_index: usize,

    // Step 4: Org selection
    pub orgs: Vec<OrgEntry>,
    pub org_index: usize,
    pub org_loading: bool,
    pub org_error: Option<String>,

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
                label: "GitHub Enterprise".to_string(),
                available: true,
            },
            ProviderChoice {
                kind: ProviderKind::GitLab,
                label: "GitLab (coming soon)".to_string(),
                available: false,
            },
            ProviderChoice {
                kind: ProviderKind::Bitbucket,
                label: "Bitbucket (coming soon)".to_string(),
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
            auth: AuthMethod::GhCli,
            api_url: None,
            token_env: None,
            prefer_ssh: true,
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
    }

    /// The 1-based step number for display (Welcome is not counted).
    pub fn step_number(&self) -> usize {
        match self.step {
            SetupStep::Welcome => 0,
            SetupStep::SelectProvider => 1,
            SetupStep::Authenticate => 2,
            SetupStep::SelectPath => 3,
            SetupStep::SelectOrgs => 4,
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
                SetupStep::SelectOrgs
            }
            SetupStep::SelectOrgs => SetupStep::Confirm,
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
            SetupStep::SelectPath => SetupStep::Authenticate,
            SetupStep::SelectOrgs => {
                self.populate_path_suggestions();
                SetupStep::SelectPath
            }
            SetupStep::Confirm => SetupStep::SelectOrgs,
            SetupStep::Complete => SetupStep::Confirm,
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_state() {
        let state = SetupState::new("~/Git-Same/GitHub");
        assert_eq!(state.step, SetupStep::SelectProvider);
        assert!(!state.should_quit);
        assert_eq!(state.base_path, "~/Git-Same/GitHub");
        assert_eq!(state.provider_choices.len(), 4);
        assert!(state.provider_choices[0].available);
        assert!(!state.provider_choices[2].available); // GitLab
        assert!(state.path_suggestions_mode);
        assert!(state.path_suggestions.is_empty());
        assert_eq!(state.tick_count, 0);
        assert!(!state.is_first_setup);
    }

    #[test]
    fn test_first_setup_starts_with_welcome() {
        let state = SetupState::with_first_setup("~/Git-Same/GitHub", true);
        assert_eq!(state.step, SetupStep::Welcome);
        assert!(state.is_first_setup);
    }

    #[test]
    fn test_non_first_setup_starts_with_provider() {
        let state = SetupState::with_first_setup("~/Git-Same/GitHub", false);
        assert_eq!(state.step, SetupStep::SelectProvider);
        assert!(!state.is_first_setup);
    }

    #[test]
    fn test_populate_path_suggestions() {
        let mut state = SetupState::new("~/test-path");
        state.populate_path_suggestions();
        // First suggestion is always the current directory (default)
        assert!(!state.path_suggestions.is_empty());
        assert_eq!(state.path_suggestions[0].path, "~/test-path");
        assert_eq!(state.path_suggestions[0].label, "current directory");
        // Last suggestion is always home
        let last = state.path_suggestions.last().unwrap();
        assert_eq!(last.path, "~");
        assert_eq!(last.label, "home");
    }

    #[test]
    fn test_tilde_collapse() {
        if let Ok(home) = std::env::var("HOME") {
            let path = format!("{}/projects", home);
            assert_eq!(super::tilde_collapse(&path), "~/projects");
        }
        assert_eq!(super::tilde_collapse("/tmp/foo"), "/tmp/foo");
    }

    #[test]
    fn test_step_navigation() {
        let mut state = SetupState::new("~/Git-Same/GitHub");
        assert_eq!(state.step, SetupStep::SelectProvider);

        state.next_step();
        assert_eq!(state.step, SetupStep::Authenticate);

        state.next_step();
        assert_eq!(state.step, SetupStep::SelectPath);

        state.prev_step();
        assert_eq!(state.step, SetupStep::Authenticate);
    }

    #[test]
    fn test_welcome_navigation() {
        let mut state = SetupState::with_first_setup("~/Git-Same/GitHub", true);
        assert_eq!(state.step, SetupStep::Welcome);

        state.next_step();
        assert_eq!(state.step, SetupStep::SelectProvider);
        assert!(!state.should_quit);
    }

    #[test]
    fn test_confirm_goes_to_complete() {
        let mut state = SetupState::new("~/Git-Same/GitHub");
        state.step = SetupStep::Confirm;
        state.next_step();
        assert_eq!(state.step, SetupStep::Complete);
        assert!(!state.should_quit);
    }

    #[test]
    fn test_complete_next_quits() {
        let mut state = SetupState::new("~/Git-Same/GitHub");
        state.step = SetupStep::Complete;
        state.next_step();
        assert!(state.should_quit);
        assert!(matches!(state.outcome, Some(SetupOutcome::Completed)));
    }

    #[test]
    fn test_selected_orgs() {
        let mut state = SetupState::new("~/Git-Same/GitHub");
        state.orgs = vec![
            OrgEntry {
                name: "org1".to_string(),
                repo_count: 5,
                selected: true,
            },
            OrgEntry {
                name: "org2".to_string(),
                repo_count: 3,
                selected: false,
            },
            OrgEntry {
                name: "org3".to_string(),
                repo_count: 8,
                selected: true,
            },
        ];
        let selected = state.selected_orgs();
        assert_eq!(selected, vec!["org1", "org3"]);
    }

    #[test]
    fn test_cancel_from_first_step() {
        let mut state = SetupState::new("~/Git-Same/GitHub");
        state.prev_step();
        assert!(state.should_quit);
        assert!(matches!(state.outcome, Some(SetupOutcome::Cancelled)));
    }

    #[test]
    fn test_cancel_from_welcome() {
        let mut state = SetupState::with_first_setup("~/Git-Same/GitHub", true);
        state.prev_step();
        assert!(state.should_quit);
        assert!(matches!(state.outcome, Some(SetupOutcome::Cancelled)));
    }

    #[test]
    fn test_step_number() {
        let mut state = SetupState::with_first_setup("~/Git-Same/GitHub", true);
        assert_eq!(state.step_number(), 0);
        state.step = SetupStep::SelectProvider;
        assert_eq!(state.step_number(), 1);
        state.step = SetupStep::Authenticate;
        assert_eq!(state.step_number(), 2);
        state.step = SetupStep::SelectPath;
        assert_eq!(state.step_number(), 3);
        state.step = SetupStep::SelectOrgs;
        assert_eq!(state.step_number(), 4);
        state.step = SetupStep::Confirm;
        assert_eq!(state.step_number(), 5);
        state.step = SetupStep::Complete;
        assert_eq!(state.step_number(), 5);
    }
}
