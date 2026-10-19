use super::*;

fn sample_check(name: &str, passed: bool, critical: bool) -> crate::checks::CheckResult {
    crate::checks::CheckResult {
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
        crate::config::Config::default_path()
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

#[test]
fn workspace_with_selected_orgs_replaces_only_orgs() {
    let mut ws = WorkspaceConfig::new_from_root(std::path::Path::new("/tmp/ws"));
    ws.orgs = vec!["old".to_string()];
    ws.username = "me".to_string();
    ws.last_synced = Some("2026-10-19T14:18:18+00:00".to_string());
    ws.filters.include_forks = true;

    let mut state = SetupState::for_org_edit(ws, "token".to_string(), None);
    state.apply_discovered_orgs(vec![
        OrgEntry {
            name: "new".to_string(),
            repo_count: 1,
            selected: true,
        },
        OrgEntry {
            name: "old".to_string(),
            repo_count: 1,
            selected: true,
        },
    ]);
    state.orgs[0].selected = true; // tick "new"

    let updated = workspace_with_selected_orgs(&state).unwrap();
    assert_eq!(updated.orgs, vec!["new", "old"]);
    assert_eq!(updated.username, "me");
    assert_eq!(
        updated.last_synced.as_deref(),
        Some("2026-10-19T14:18:18+00:00")
    );
    assert!(updated.filters.include_forks);
}

#[test]
fn workspace_with_selected_orgs_rejects_empty_selection_and_non_edit_state() {
    let ws = WorkspaceConfig::new_from_root(std::path::Path::new("/tmp/ws"));
    let state = SetupState::for_org_edit(ws, "token".to_string(), None);
    assert!(workspace_with_selected_orgs(&state).is_err());

    let state = SetupState::new("~/Git-Same/GitHub");
    assert!(workspace_with_selected_orgs(&state).is_err());
}
