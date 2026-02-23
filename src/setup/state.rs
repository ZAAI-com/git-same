//! Setup wizard state (the "Model" in Elm architecture).

use crate::config::{AuthMethod, WorkspaceProvider};
use crate::types::ProviderKind;

/// Which step of the wizard is active.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetupStep {
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

impl SetupState {
    /// Create initial wizard state.
    pub fn new(default_base_path: &str) -> Self {
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

        Self {
            step: SetupStep::SelectProvider,
            should_quit: false,
            outcome: None,
            provider_choices,
            provider_index: 0,
            auth_status: AuthStatus::Pending,
            username: None,
            auth_token: None,
            base_path,
            path_cursor,
            orgs: Vec::new(),
            org_index: 0,
            org_loading: false,
            org_error: None,
            workspace_name: String::new(),
            name_editing: false,
            error_message: None,
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

    /// Move to the next step.
    pub fn next_step(&mut self) {
        self.error_message = None;
        self.step = match self.step {
            SetupStep::SelectProvider => SetupStep::Authenticate,
            SetupStep::Authenticate => SetupStep::SelectPath,
            SetupStep::SelectPath => {
                // Derive workspace name from base_path
                let path = std::path::Path::new(&self.base_path);
                self.workspace_name = crate::config::WorkspaceManager::name_from_path(path);
                SetupStep::SelectOrgs
            }
            SetupStep::SelectOrgs => SetupStep::Confirm,
            SetupStep::Confirm => {
                self.outcome = Some(SetupOutcome::Completed);
                self.should_quit = true;
                SetupStep::Confirm
            }
        };
    }

    /// Move to the previous step.
    pub fn prev_step(&mut self) {
        self.error_message = None;
        self.step = match self.step {
            SetupStep::SelectProvider => {
                self.outcome = Some(SetupOutcome::Cancelled);
                self.should_quit = true;
                SetupStep::SelectProvider
            }
            SetupStep::Authenticate => SetupStep::SelectProvider,
            SetupStep::SelectPath => SetupStep::Authenticate,
            SetupStep::SelectOrgs => SetupStep::SelectPath,
            SetupStep::Confirm => SetupStep::SelectOrgs,
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_state() {
        let state = SetupState::new("~/github");
        assert_eq!(state.step, SetupStep::SelectProvider);
        assert!(!state.should_quit);
        assert_eq!(state.base_path, "~/github");
        assert_eq!(state.provider_choices.len(), 4);
        assert!(state.provider_choices[0].available);
        assert!(!state.provider_choices[2].available); // GitLab
    }

    #[test]
    fn test_step_navigation() {
        let mut state = SetupState::new("~/github");
        assert_eq!(state.step, SetupStep::SelectProvider);

        state.next_step();
        assert_eq!(state.step, SetupStep::Authenticate);

        state.next_step();
        assert_eq!(state.step, SetupStep::SelectPath);

        state.prev_step();
        assert_eq!(state.step, SetupStep::Authenticate);
    }

    #[test]
    fn test_selected_orgs() {
        let mut state = SetupState::new("~/github");
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
        let mut state = SetupState::new("~/github");
        state.prev_step();
        assert!(state.should_quit);
        assert!(matches!(state.outcome, Some(SetupOutcome::Cancelled)));
    }
}
