use super::*;
use crate::setup::state::SetupStep;

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
async fn right_advances_from_provider_step() {
    let mut state = SetupState::new("~/Git-Same/GitHub");
    assert_eq!(state.step, SetupStep::SelectProvider);

    handle_key(
        &mut state,
        KeyEvent::new(KeyCode::Right, KeyModifiers::NONE),
    )
    .await;

    assert_eq!(state.step, SetupStep::Authenticate);
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

    let children: Vec<_> = state
        .path_browse_entries
        .iter()
        .filter(|entry| {
            entry.depth == 2
                && entry.path.starts_with(&format!(
                    "{}/",
                    super::tilde_collapse(&temp.path().to_string_lossy())
                ))
        })
        .map(|entry| entry.label.clone())
        .collect();
    assert_eq!(children.len(), 150);
    assert_eq!(children.first().map(String::as_str), Some("d000/"));
    assert_eq!(children.last().map(String::as_str), Some("d149/"));
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
