use super::*;
use crate::setup::state::{PathSuggestion, SetupStep};

fn cwd_collapsed() -> String {
    super::tilde_collapse(&std::env::current_dir().unwrap().to_string_lossy())
}

fn tempdir_in_cwd(prefix: &str) -> tempfile::TempDir {
    let cwd = std::env::current_dir().unwrap();
    tempfile::Builder::new()
        .prefix(prefix)
        .tempdir_in(cwd)
        .unwrap()
}

fn find_entry_index(state: &SetupState, path: &std::path::Path) -> usize {
    let wanted = super::tilde_collapse(&path.to_string_lossy());
    state
        .path_browse_entries
        .iter()
        .position(|entry| entry.path == wanted)
        .expect("expected path to be listed in popup tree")
}

#[tokio::test]
async fn q_quits_setup_wizard() {
    let mut state = SetupState::new("~/Git-Same/GitHub");
    state.step = SetupStep::SelectProvider;

    handle_key(
        &mut state,
        KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE),
    )
    .await;

    assert!(state.should_quit);
    assert!(matches!(state.outcome, Some(SetupOutcome::Cancelled)));
}

#[tokio::test]
async fn esc_cancels_setup_from_any_step() {
    let mut state = SetupState::new("~/Git-Same/GitHub");
    state.step = SetupStep::SelectOrgs;
    state.org_loading = false;

    handle_key(&mut state, KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)).await;

    assert!(state.should_quit);
    assert!(matches!(state.outcome, Some(SetupOutcome::Cancelled)));
}

#[tokio::test]
async fn left_moves_to_previous_step() {
    let mut state = SetupState::new("~/Git-Same/GitHub");
    state.step = SetupStep::SelectOrgs;
    state.org_loading = false;

    handle_key(&mut state, KeyEvent::new(KeyCode::Left, KeyModifiers::NONE)).await;

    assert_eq!(state.step, SetupStep::Authenticate);
}

#[tokio::test]
async fn org_loading_ignores_non_null_keys() {
    let mut state = SetupState::new("~/Git-Same/GitHub");
    state.step = SetupStep::SelectOrgs;
    state.org_loading = true;
    state.org_error = None;
    state.auth_token = None;

    handle_key(&mut state, KeyEvent::new(KeyCode::Down, KeyModifiers::NONE)).await;

    assert!(state.org_loading);
    assert!(state.org_error.is_none());
}

#[tokio::test]
async fn right_advances_from_provider_step() {
    let mut state = SetupState::new("~/Git-Same/GitHub");
    state.step = SetupStep::SelectProvider;
    assert_eq!(state.step, SetupStep::SelectProvider);

    handle_key(
        &mut state,
        KeyEvent::new(KeyCode::Right, KeyModifiers::NONE),
    )
    .await;

    assert_eq!(state.step, SetupStep::Authenticate);
}

#[tokio::test]
async fn left_in_select_path_returns_to_orgs_step() {
    let mut state = SetupState::new("~/Git-Same/GitHub");
    state.step = SetupStep::SelectPath;
    state.path_browse_mode = false;
    state.path_suggestions_mode = false;

    handle_key(&mut state, KeyEvent::new(KeyCode::Left, KeyModifiers::NONE)).await;

    assert_eq!(state.step, SetupStep::SelectOrgs);
    assert!(!state.path_browse_mode);
}

#[tokio::test]
async fn typing_does_not_edit_base_path_in_select_path_step() {
    let mut state = SetupState::new("~/Git-Same/GitHub");
    state.step = SetupStep::SelectPath;
    let original = state.base_path.clone();

    for key in [
        KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE),
        KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE),
        KeyEvent::new(KeyCode::Delete, KeyModifiers::NONE),
        KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE),
        KeyEvent::new(KeyCode::Left, KeyModifiers::NONE),
        KeyEvent::new(KeyCode::Right, KeyModifiers::NONE),
    ] {
        handle_key(&mut state, key).await;
    }

    assert_eq!(state.base_path, original);
    assert_eq!(state.step, SetupStep::SelectPath);
}

#[tokio::test]
async fn enter_in_suggestions_mode_does_not_change_base_path() {
    let mut state = SetupState::new("~/Git-Same/GitHub");
    state.step = SetupStep::SelectPath;
    state.path_suggestions_mode = true;
    state.path_suggestions = vec![
        PathSuggestion {
            path: "~/Git-Same/GitHub".to_string(),
            label: "terminal folder".to_string(),
        },
        PathSuggestion {
            path: "~/Developer".to_string(),
            label: "other".to_string(),
        },
    ];
    state.path_suggestion_index = 1;
    let original = state.base_path.clone();

    handle_key(
        &mut state,
        KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
    )
    .await;

    assert_eq!(state.base_path, original);
    assert_eq!(state.step, SetupStep::Confirm);
}

#[tokio::test]
async fn b_opens_path_browser_from_suggestions_mode() {
    let temp = tempdir_in_cwd("gisa-path-browse-");
    std::fs::create_dir_all(temp.path().join("child")).unwrap();

    let mut state = SetupState::new(&temp.path().to_string_lossy());
    state.step = SetupStep::SelectPath;
    state.populate_path_suggestions();
    state.base_path = temp.path().to_string_lossy().to_string();
    state.path_cursor = state.base_path.len();

    handle_key(
        &mut state,
        KeyEvent::new(KeyCode::Char('b'), KeyModifiers::NONE),
    )
    .await;

    assert!(state.path_browse_mode);
    assert_eq!(state.step, SetupStep::SelectPath);
    assert_eq!(state.path_browse_index, 0);
    assert_eq!(state.path_browse_current_dir, cwd_collapsed());
    assert_eq!(state.path_browse_entries[0].depth, 0);
    assert!(state.path_browse_entries.iter().any(|entry| entry.path
        == super::tilde_collapse(&temp.path().to_string_lossy())
        && entry.depth == 1));
}

#[tokio::test]
async fn left_on_root_moves_popup_to_parent_directory() {
    let mut state = SetupState::new("~/Git-Same/GitHub");
    state.step = SetupStep::SelectPath;

    handle_key(
        &mut state,
        KeyEvent::new(KeyCode::Char('b'), KeyModifiers::CONTROL),
    )
    .await;
    assert!(state.path_browse_mode);
    assert_eq!(state.path_browse_index, 0);

    let root_before =
        std::path::PathBuf::from(shellexpand::tilde(&state.path_browse_entries[0].path).as_ref());
    let Some(parent_before) = root_before.parent().map(std::path::Path::to_path_buf) else {
        // Nothing above `/` on this platform.
        return;
    };

    handle_key(&mut state, KeyEvent::new(KeyCode::Left, KeyModifiers::NONE)).await;

    let expected = super::tilde_collapse(&parent_before.to_string_lossy());
    assert_eq!(state.path_browse_index, 0);
    assert_eq!(state.path_browse_current_dir, expected);
}

#[tokio::test]
async fn right_in_path_browse_mode_navigates_tree_without_advancing_step() {
    let temp = tempdir_in_cwd("gisa-path-nav-");
    let alpha = temp.path().join("alpha");
    std::fs::create_dir_all(&alpha).unwrap();
    let expected = super::tilde_collapse(&alpha.to_string_lossy());

    let mut state = SetupState::new(&temp.path().to_string_lossy());
    state.step = SetupStep::SelectPath;
    state.path_suggestions_mode = false;
    state.base_path = temp.path().to_string_lossy().to_string();
    state.path_cursor = state.base_path.len();

    handle_key(
        &mut state,
        KeyEvent::new(KeyCode::Char('b'), KeyModifiers::CONTROL),
    )
    .await;
    assert!(state.path_browse_mode);
    state.path_browse_index = find_entry_index(&state, temp.path());

    handle_key(
        &mut state,
        KeyEvent::new(KeyCode::Right, KeyModifiers::NONE),
    )
    .await;
    assert!(state.path_browse_entries[state.path_browse_index].expanded);

    assert_eq!(state.step, SetupStep::SelectPath);
    assert!(state.path_browse_mode);

    handle_key(
        &mut state,
        KeyEvent::new(KeyCode::Right, KeyModifiers::NONE),
    )
    .await;
    assert_eq!(state.path_browse_current_dir, expected);
}

#[tokio::test]
async fn enter_in_browse_mode_sets_path_and_closes_popup() {
    let temp = tempdir_in_cwd("gisa-path-enter-");
    let alpha = temp.path().join("alpha");
    std::fs::create_dir_all(&alpha).unwrap();
    let expected = super::tilde_collapse(&alpha.to_string_lossy());

    let mut state = SetupState::new(&temp.path().to_string_lossy());
    state.step = SetupStep::SelectPath;
    state.path_suggestions_mode = false;
    state.base_path = temp.path().to_string_lossy().to_string();
    state.path_cursor = state.base_path.len();

    handle_key(
        &mut state,
        KeyEvent::new(KeyCode::Char('b'), KeyModifiers::CONTROL),
    )
    .await;
    state.path_browse_index = find_entry_index(&state, temp.path());
    handle_key(
        &mut state,
        KeyEvent::new(KeyCode::Right, KeyModifiers::NONE),
    )
    .await;
    handle_key(
        &mut state,
        KeyEvent::new(KeyCode::Right, KeyModifiers::NONE),
    )
    .await;

    handle_key(
        &mut state,
        KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
    )
    .await;

    assert!(!state.path_browse_mode);
    assert_eq!(state.step, SetupStep::SelectPath);
    assert_eq!(state.base_path, expected);

    handle_key(
        &mut state,
        KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
    )
    .await;
    assert_eq!(state.step, SetupStep::Confirm);
}

#[tokio::test]
async fn esc_in_popup_only_closes_popup() {
    let temp = tempfile::tempdir().unwrap();
    let mut state = SetupState::new(&temp.path().to_string_lossy());
    state.step = SetupStep::SelectPath;
    state.path_suggestions_mode = false;
    state.base_path = temp.path().to_string_lossy().to_string();
    state.path_cursor = state.base_path.len();

    handle_key(
        &mut state,
        KeyEvent::new(KeyCode::Char('b'), KeyModifiers::CONTROL),
    )
    .await;
    assert!(state.path_browse_mode);

    handle_key(&mut state, KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)).await;

    assert!(!state.path_browse_mode);
    assert_eq!(state.step, SetupStep::SelectPath);
    assert!(!state.should_quit);
}

#[tokio::test]
async fn left_moves_to_parent_and_then_collapses() {
    let temp = tempdir_in_cwd("gisa-path-left-");
    let alpha = temp.path().join("alpha");
    let nested = alpha.join("nested");
    std::fs::create_dir_all(&nested).unwrap();
    let parent = super::tilde_collapse(&temp.path().to_string_lossy());

    let mut state = SetupState::new(&temp.path().to_string_lossy());
    state.step = SetupStep::SelectPath;
    state.path_suggestions_mode = false;
    state.base_path = temp.path().to_string_lossy().to_string();
    state.path_cursor = state.base_path.len();

    handle_key(
        &mut state,
        KeyEvent::new(KeyCode::Char('b'), KeyModifiers::CONTROL),
    )
    .await;
    state.path_browse_index = find_entry_index(&state, temp.path());
    handle_key(
        &mut state,
        KeyEvent::new(KeyCode::Right, KeyModifiers::NONE),
    )
    .await;
    handle_key(
        &mut state,
        KeyEvent::new(KeyCode::Right, KeyModifiers::NONE),
    )
    .await;
    assert_eq!(
        state.path_browse_current_dir,
        super::tilde_collapse(&alpha.to_string_lossy())
    );

    handle_key(&mut state, KeyEvent::new(KeyCode::Left, KeyModifiers::NONE)).await;
    assert_eq!(state.path_browse_current_dir, parent);

    let before = state.path_browse_entries.len();
    handle_key(&mut state, KeyEvent::new(KeyCode::Left, KeyModifiers::NONE)).await;
    assert!(state.path_browse_entries.len() < before);
    let parent_index = find_entry_index(&state, temp.path());
    assert!(!state.path_browse_entries[parent_index].expanded);
}

#[tokio::test]
async fn right_on_leaf_does_not_change_selection_until_enter() {
    let leaf_temp = tempdir_in_cwd("gisa-path-leaf-");
    let expected = super::tilde_collapse(&leaf_temp.path().to_string_lossy());

    let mut state = SetupState::new(&leaf_temp.path().to_string_lossy());
    state.step = SetupStep::SelectPath;
    state.path_suggestions_mode = false;
    state.base_path = leaf_temp.path().to_string_lossy().to_string();
    state.path_cursor = state.base_path.len();

    handle_key(
        &mut state,
        KeyEvent::new(KeyCode::Char('b'), KeyModifiers::CONTROL),
    )
    .await;
    state.path_browse_index = find_entry_index(&state, leaf_temp.path());
    state.path_browse_current_dir = expected.clone();
    assert_eq!(state.path_browse_current_dir, expected);

    let selected_before = state.path_browse_index;
    handle_key(
        &mut state,
        KeyEvent::new(KeyCode::Right, KeyModifiers::NONE),
    )
    .await;
    assert_eq!(state.path_browse_index, selected_before);
    assert_eq!(state.path_browse_current_dir, expected);
    assert!(state.path_browse_mode);
}

#[tokio::test]
async fn very_large_directory_list_is_loaded() {
    let temp = tempdir_in_cwd("gisa-path-many-");
    for i in 0..150 {
        std::fs::create_dir_all(temp.path().join(format!("d{i:03}"))).unwrap();
    }

    let mut state = SetupState::new(&temp.path().to_string_lossy());
    state.step = SetupStep::SelectPath;
    state.path_suggestions_mode = false;
    state.base_path = temp.path().to_string_lossy().to_string();
    state.path_cursor = state.base_path.len();

    handle_key(
        &mut state,
        KeyEvent::new(KeyCode::Char('b'), KeyModifiers::CONTROL),
    )
    .await;
    assert!(state.path_browse_error.is_none());
    state.path_browse_index = find_entry_index(&state, temp.path());
    handle_key(
        &mut state,
        KeyEvent::new(KeyCode::Right, KeyModifiers::NONE),
    )
    .await;

    let prefix = format!(
        "{}{}",
        super::tilde_collapse(&temp.path().to_string_lossy()),
        std::path::MAIN_SEPARATOR
    );
    let children: Vec<_> = state
        .path_browse_entries
        .iter()
        .filter(|entry| entry.depth == 2 && entry.path.starts_with(&prefix))
        .map(|entry| entry.label.clone())
        .collect();
    assert_eq!(children.len(), 150);
    assert_eq!(children.first().map(String::as_str), Some("d000/"));
    assert_eq!(children.last().map(String::as_str), Some("d149/"));
}

// --- Tests for match-guard refactoring introduced in this PR ---

// ── handle_requirements ──────────────────────────────────────────────────────

/// Enter while checks are still loading must NOT advance the step (match guard:
/// `KeyCode::Enter if !state.checks_loading && state.requirements_passed()`).
#[tokio::test]
async fn enter_while_checks_loading_does_not_advance_requirements_step() {
    let mut state = SetupState::new("~/Git-Same/GitHub");
    state.step = SetupStep::Requirements;
    state.checks_loading = true;
    // Plant a passing result to confirm loading flag is the only blocker.
    state.check_results = vec![crate::checks::CheckResult {
        name: "git".to_string(),
        passed: true,
        message: "git 2.40".to_string(),
        suggestion: None,
        critical: true,
    }];

    handle_key(
        &mut state,
        KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
    )
    .await;

    assert_eq!(state.step, SetupStep::Requirements);
}

/// Enter when requirements are NOT met (no check_results) must not advance.
#[tokio::test]
async fn enter_when_requirements_not_passed_does_not_advance() {
    let mut state = SetupState::new("~/Git-Same/GitHub");
    state.step = SetupStep::Requirements;
    state.checks_loading = false;
    // check_results is empty → requirements_passed() returns false.

    handle_key(
        &mut state,
        KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
    )
    .await;

    assert_eq!(state.step, SetupStep::Requirements);
}

/// Enter when requirements have a failed critical check must not advance.
#[tokio::test]
async fn enter_when_critical_check_failed_does_not_advance() {
    let mut state = SetupState::new("~/Git-Same/GitHub");
    state.step = SetupStep::Requirements;
    state.checks_loading = false;
    state.check_results = vec![crate::checks::CheckResult {
        name: "git".to_string(),
        passed: false,
        message: "not found".to_string(),
        suggestion: Some("install git".to_string()),
        critical: true,
    }];

    handle_key(
        &mut state,
        KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
    )
    .await;

    assert_eq!(state.step, SetupStep::Requirements);
}

/// Enter when all critical checks pass and not loading must advance to next step.
#[tokio::test]
async fn enter_when_requirements_passed_and_not_loading_advances_step() {
    let mut state = SetupState::new("~/Git-Same/GitHub");
    state.step = SetupStep::Requirements;
    state.checks_loading = false;
    state.check_results = vec![crate::checks::CheckResult {
        name: "git".to_string(),
        passed: true,
        message: "git 2.40".to_string(),
        suggestion: None,
        critical: true,
    }];

    handle_key(
        &mut state,
        KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
    )
    .await;

    assert_eq!(state.step, SetupStep::SelectProvider);
}

// ── handle_provider ───────────────────────────────────────────────────────────

/// Up at provider_index == 0 must not decrement (underflow / out-of-bounds).
#[tokio::test]
async fn up_at_first_provider_does_not_move() {
    let mut state = SetupState::new("~/Git-Same/GitHub");
    state.step = SetupStep::SelectProvider;
    state.provider_index = 0;

    handle_key(&mut state, KeyEvent::new(KeyCode::Up, KeyModifiers::NONE)).await;

    assert_eq!(state.provider_index, 0);
}

/// Down at the last provider choice must not move past the end.
#[tokio::test]
async fn down_at_last_provider_does_not_move() {
    let mut state = SetupState::new("~/Git-Same/GitHub");
    state.step = SetupStep::SelectProvider;
    let last = state.provider_choices.len() - 1;
    state.provider_index = last;

    handle_key(&mut state, KeyEvent::new(KeyCode::Down, KeyModifiers::NONE)).await;

    assert_eq!(state.provider_index, last);
}

/// Enter when the selected provider is NOT available must not advance.
/// (index 1 = GitHubEnterprise, available=false in default state)
#[tokio::test]
async fn enter_on_unavailable_provider_does_not_advance() {
    let mut state = SetupState::new("~/Git-Same/GitHub");
    state.step = SetupStep::SelectProvider;
    // Choose a provider marked unavailable.
    state.provider_index = 1; // GitHubEnterprise, available=false

    handle_key(
        &mut state,
        KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
    )
    .await;

    assert_eq!(state.step, SetupStep::SelectProvider);
}

/// Enter on an available provider (GitHub, index 0) must advance to Authenticate.
#[tokio::test]
async fn enter_on_available_provider_advances_to_authenticate() {
    let mut state = SetupState::new("~/Git-Same/GitHub");
    state.step = SetupStep::SelectProvider;
    state.provider_index = 0; // GitHub, available=true

    handle_key(
        &mut state,
        KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
    )
    .await;

    assert_eq!(state.step, SetupStep::Authenticate);
}

// ── handle_orgs ───────────────────────────────────────────────────────────────

/// Up at org_index == 0 must not move (guard: `org_index > 0`).
#[tokio::test]
async fn up_at_first_org_does_not_move() {
    let mut state = SetupState::new("~/Git-Same/GitHub");
    state.step = SetupStep::SelectOrgs;
    state.org_loading = false;
    state.orgs = vec![
        crate::setup::state::OrgEntry {
            name: "alpha".to_string(),
            repo_count: 5,
            selected: true,
        },
        crate::setup::state::OrgEntry {
            name: "beta".to_string(),
            repo_count: 3,
            selected: false,
        },
    ];
    state.org_index = 0;

    handle_key(&mut state, KeyEvent::new(KeyCode::Up, KeyModifiers::NONE)).await;

    assert_eq!(state.org_index, 0);
}

/// Down at the last org must not move past the end.
#[tokio::test]
async fn down_at_last_org_does_not_move() {
    let mut state = SetupState::new("~/Git-Same/GitHub");
    state.step = SetupStep::SelectOrgs;
    state.org_loading = false;
    state.orgs = vec![
        crate::setup::state::OrgEntry {
            name: "alpha".to_string(),
            repo_count: 5,
            selected: true,
        },
        crate::setup::state::OrgEntry {
            name: "beta".to_string(),
            repo_count: 3,
            selected: false,
        },
    ];
    state.org_index = 1; // last

    handle_key(&mut state, KeyEvent::new(KeyCode::Down, KeyModifiers::NONE)).await;

    assert_eq!(state.org_index, 1);
}

/// Space when orgs list is empty must not panic (guard: `!state.orgs.is_empty()`).
#[tokio::test]
async fn space_with_empty_orgs_does_not_panic() {
    let mut state = SetupState::new("~/Git-Same/GitHub");
    state.step = SetupStep::SelectOrgs;
    state.org_loading = false;
    // orgs is empty by default

    // Should complete without panicking.
    handle_key(
        &mut state,
        KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE),
    )
    .await;
}

/// Space on a non-empty list toggles the org selection.
#[tokio::test]
async fn space_toggles_org_selection() {
    let mut state = SetupState::new("~/Git-Same/GitHub");
    state.step = SetupStep::SelectOrgs;
    state.org_loading = false;
    state.orgs = vec![crate::setup::state::OrgEntry {
        name: "acme".to_string(),
        repo_count: 10,
        selected: false,
    }];
    state.org_index = 0;

    handle_key(
        &mut state,
        KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE),
    )
    .await;
    assert!(state.orgs[0].selected);

    handle_key(
        &mut state,
        KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE),
    )
    .await;
    assert!(!state.orgs[0].selected);
}

// ── handle_path_browse ────────────────────────────────────────────────────────

/// Up at path_browse_index == 0 must not move (underflow guard).
#[tokio::test]
async fn up_at_first_browse_entry_does_not_move() {
    let temp = tempdir_in_cwd("gisa-hbrowse-up-");
    let alpha = temp.path().join("a-dir");
    std::fs::create_dir_all(&alpha).unwrap();

    let mut state = SetupState::new(&temp.path().to_string_lossy());
    state.step = SetupStep::SelectPath;
    state.path_suggestions_mode = false;
    state.base_path = temp.path().to_string_lossy().to_string();

    // Open browse mode to populate entries.
    handle_key(
        &mut state,
        KeyEvent::new(KeyCode::Char('b'), KeyModifiers::CONTROL),
    )
    .await;
    assert!(state.path_browse_mode);
    state.path_browse_index = 0;

    handle_key(&mut state, KeyEvent::new(KeyCode::Up, KeyModifiers::NONE)).await;

    assert_eq!(state.path_browse_index, 0);
}

/// Down at the last browse entry must not move past the end.
#[tokio::test]
async fn down_at_last_browse_entry_does_not_move() {
    let temp = tempdir_in_cwd("gisa-hbrowse-dn-");
    let alpha = temp.path().join("only-child");
    std::fs::create_dir_all(&alpha).unwrap();

    let mut state = SetupState::new(&temp.path().to_string_lossy());
    state.step = SetupStep::SelectPath;
    state.path_suggestions_mode = false;
    state.base_path = temp.path().to_string_lossy().to_string();

    handle_key(
        &mut state,
        KeyEvent::new(KeyCode::Char('b'), KeyModifiers::CONTROL),
    )
    .await;
    assert!(state.path_browse_mode);

    // Navigate to the last entry.
    let last = state.path_browse_entries.len() - 1;
    state.path_browse_index = last;

    handle_key(&mut state, KeyEvent::new(KeyCode::Down, KeyModifiers::NONE)).await;

    assert_eq!(state.path_browse_index, last);
}

#[cfg(unix)]
#[tokio::test]
async fn unreadable_directory_surfaces_inline_error() {
    use std::os::unix::fs::PermissionsExt;

    let temp = tempfile::tempdir().unwrap();
    let locked = temp.path().join("locked");
    std::fs::create_dir_all(&locked).unwrap();
    let mut perms = std::fs::metadata(&locked).unwrap().permissions();
    perms.set_mode(0o000);
    std::fs::set_permissions(&locked, perms).unwrap();

    if std::fs::read_dir(&locked).is_ok() {
        let mut reset = std::fs::metadata(&locked).unwrap().permissions();
        reset.set_mode(0o700);
        std::fs::set_permissions(&locked, reset).unwrap();
        return;
    }

    let mut state = SetupState::new("~/Git-Same/GitHub");
    set_browse_root(&mut state, locked.clone());
    assert!(state.path_browse_error.is_some());

    let mut reset = std::fs::metadata(&locked).unwrap().permissions();
    reset.set_mode(0o700);
    std::fs::set_permissions(&locked, reset).unwrap();
}
