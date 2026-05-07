use super::*;

fn sample_check(name: &str, passed: bool, critical: bool) -> git_same_core::checks::CheckResult {
    git_same_core::checks::CheckResult {
        name: name.to_string(),
        passed,
        message: "ok".to_string(),
        suggestion: None,
        critical,
    }
}

#[test]
fn maybe_start_requirements_checks_sets_expected_state() {
    let mut state = SetupState::new("~/Git-Same/GitHub");
    assert_eq!(state.step, SetupStep::Requirements);
    assert!(!state.checks_triggered);
    assert!(!state.checks_loading);

    assert!(maybe_start_requirements_checks(&mut state));
    assert!(state.checks_triggered);
    assert!(state.checks_loading);
    assert_eq!(
        state.config_path_display,
        git_same_core::config::Config::default_path()
            .ok()
            .map(|p| p.display().to_string())
    );
}

#[test]
fn maybe_start_requirements_checks_noops_when_not_requirements_or_already_triggered() {
    let mut state = SetupState::new("~/Git-Same/GitHub");
    state.step = SetupStep::SelectProvider;
    assert!(!maybe_start_requirements_checks(&mut state));
    assert!(!state.checks_triggered);
    assert!(!state.checks_loading);

    state.step = SetupStep::Requirements;
    state.checks_triggered = true;
    assert!(!maybe_start_requirements_checks(&mut state));
    assert!(state.checks_triggered);
}

#[test]
fn apply_requirements_check_results_updates_state_and_clears_loading() {
    let mut state = SetupState::new("~/Git-Same/GitHub");
    state.checks_loading = true;
    state.check_results = vec![sample_check("old", false, true)];

    let results = vec![
        sample_check("Git", true, true),
        sample_check("SSH GitHub", false, false),
    ];
    apply_requirements_check_results(&mut state, results);

    assert!(!state.checks_loading);
    assert_eq!(state.check_results.len(), 2);
    assert_eq!(state.check_results[0].name, "Git");
    assert!(state.check_results[0].passed);
    assert_eq!(state.check_results[1].name, "SSH GitHub");
    assert!(!state.check_results[1].passed);
}

#[tokio::test]
async fn standalone_requirements_step_auto_runs_checks_end_to_end() {
    let mut state = SetupState::new("~/Git-Same/GitHub");
    assert_eq!(state.step, SetupStep::Requirements);
    assert!(state.check_results.is_empty());
    assert!(!state.checks_loading);

    assert!(maybe_start_requirements_checks(&mut state));
    run_requirements_checks(&mut state).await;

    assert!(state.checks_triggered);
    assert!(!state.checks_loading);
    assert!(!state.check_results.is_empty());
    assert_eq!(state.step, SetupStep::Requirements);
}
