use super::*;

#[test]
fn test_new_state() {
    let state = SetupState::new("~/Git-Same/GitHub");
    assert_eq!(state.step, SetupStep::SelectProvider);
    assert!(!state.should_quit);
    assert_eq!(state.base_path, "~/Git-Same/GitHub");
    assert_eq!(state.provider_choices.len(), 6);
    assert!(state.provider_choices[0].available);
    assert!(!state.provider_choices[2].available); // GitLab
    assert!(state.path_suggestions_mode);
    assert!(!state.path_browse_mode);
    assert!(state.path_browse_entries.is_empty());
    assert!(!state.path_browse_show_hidden);
    assert!(state.path_browse_error.is_none());
    assert!(state.path_browse_info.is_none());
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
    assert_eq!(state.step, SetupStep::SelectOrgs);

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
    state.step = SetupStep::SelectOrgs;
    assert_eq!(state.step_number(), 3);
    state.step = SetupStep::SelectPath;
    assert_eq!(state.step_number(), 4);
    state.step = SetupStep::Confirm;
    assert_eq!(state.step_number(), 5);
    state.step = SetupStep::Complete;
    assert_eq!(state.step_number(), 5);
}
