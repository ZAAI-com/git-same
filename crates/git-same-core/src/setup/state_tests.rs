use super::*;

#[test]
fn test_new_state() {
    let state = SetupState::new("~/Git-Same/GitHub");
    assert_eq!(state.step, SetupStep::Requirements);
    assert!(!state.should_quit);
    assert_eq!(state.base_path, "~/Git-Same/GitHub");
    assert_eq!(state.provider_choices.len(), 6);
    assert!(state.provider_choices[0].available);
    assert!(!state.provider_choices[2].available); // GitLab
    assert!(!state.path_suggestions_mode);
    assert!(!state.path_browse_mode);
    assert!(state.path_browse_entries.is_empty());
    assert!(!state.path_browse_show_hidden);
    assert!(state.path_browse_error.is_none());
    assert!(state.path_browse_info.is_none());
    assert!(state.path_suggestions.is_empty());
    assert_eq!(state.tick_count, 0);
    assert!(!state.is_first_setup);
    assert!(!state.checks_triggered);
    assert!(!state.checks_loading);
    assert!(!state.config_was_created);
}

#[test]
fn test_first_setup_starts_with_requirements() {
    let state = SetupState::with_first_setup("~/Git-Same/GitHub", true);
    assert_eq!(state.step, SetupStep::Requirements);
    assert!(state.is_first_setup);
}

#[test]
fn test_non_first_setup_starts_with_requirements() {
    let state = SetupState::with_first_setup("~/Git-Same/GitHub", false);
    assert_eq!(state.step, SetupStep::Requirements);
    assert!(!state.is_first_setup);
}

#[test]
fn test_populate_path_suggestions() {
    let mut state = SetupState::new("~/test-path");
    state.populate_path_suggestions();
    assert_eq!(state.path_suggestions.len(), 1);
    assert_eq!(state.path_suggestions[0].path, "~/test-path");
    assert_eq!(state.path_suggestions[0].label, "terminal folder");
    assert!(!state.path_suggestions_mode);
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
    assert_eq!(state.step, SetupStep::Requirements);

    state.next_step();
    assert_eq!(state.step, SetupStep::SelectProvider);

    state.next_step();
    assert_eq!(state.step, SetupStep::Authenticate);

    state.prev_step();
    assert_eq!(state.step, SetupStep::SelectProvider);
}

#[test]
fn test_requirements_to_provider() {
    let mut state = SetupState::with_first_setup("~/Git-Same/GitHub", true);
    assert_eq!(state.step, SetupStep::Requirements);

    state.next_step();
    assert_eq!(state.step, SetupStep::SelectProvider);
    assert!(!state.should_quit);
}

#[test]
fn test_provider_back_to_requirements() {
    let mut state = SetupState::new("~/Git-Same/GitHub");
    state.step = SetupStep::SelectProvider;
    state.prev_step();
    assert_eq!(state.step, SetupStep::Requirements);
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
fn test_cancel_from_requirements() {
    let mut state = SetupState::new("~/Git-Same/GitHub");
    state.prev_step();
    assert!(state.should_quit);
    assert!(matches!(state.outcome, Some(SetupOutcome::Cancelled)));
}

#[test]
fn test_requirements_passed_all_critical_pass() {
    let mut state = SetupState::new("~/Git-Same/GitHub");
    state.check_results = vec![
        crate::checks::CheckResult {
            name: "Git".to_string(),
            passed: true,
            message: "ok".to_string(),
            suggestion: None,
            critical: true,
        },
        crate::checks::CheckResult {
            name: "SSH".to_string(),
            passed: false,
            message: "not found".to_string(),
            suggestion: None,
            critical: false, // warning only
        },
    ];
    assert!(state.requirements_passed());
}

#[test]
fn test_requirements_passed_critical_fail() {
    let mut state = SetupState::new("~/Git-Same/GitHub");
    state.check_results = vec![crate::checks::CheckResult {
        name: "Git".to_string(),
        passed: false,
        message: "not found".to_string(),
        suggestion: None,
        critical: true,
    }];
    assert!(!state.requirements_passed());
}

#[test]
fn test_requirements_passed_empty_is_false() {
    let state = SetupState::new("~/Git-Same/GitHub");
    assert!(!state.requirements_passed());
}

#[test]
fn test_step_number() {
    let mut state = SetupState::new("~/Git-Same/GitHub");
    assert_eq!(state.step_number(), 1); // Requirements
    state.step = SetupStep::SelectProvider;
    assert_eq!(state.step_number(), 2);
    state.step = SetupStep::Authenticate;
    assert_eq!(state.step_number(), 3);
    state.step = SetupStep::SelectOrgs;
    assert_eq!(state.step_number(), 4);
    state.step = SetupStep::SelectPath;
    assert_eq!(state.step_number(), 5);
    state.step = SetupStep::Confirm;
    assert_eq!(state.step_number(), 6);
    state.step = SetupStep::Complete;
    assert_eq!(state.step_number(), 6);
}

#[test]
fn test_total_steps_is_six() {
    assert_eq!(SetupState::TOTAL_STEPS, 6);
}
