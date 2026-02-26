use super::*;
use crate::setup::state::SetupStep;

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
async fn b_opens_path_browser_from_suggestions_mode() {
    let temp = tempfile::tempdir().unwrap();
    let child = temp.path().join("child");
    std::fs::create_dir_all(&child).unwrap();

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
    assert_eq!(
        state.path_browse_current_dir,
        super::tilde_collapse(&temp.path().to_string_lossy())
    );
    assert!(state
        .path_browse_entries
        .iter()
        .any(|entry| entry.path == super::tilde_collapse(&child.to_string_lossy())));
}

#[tokio::test]
async fn enter_opens_selected_directory_without_confirming_step() {
    let temp = tempfile::tempdir().unwrap();
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

    let alpha_index = state
        .path_browse_entries
        .iter()
        .position(|entry| entry.path == expected)
        .expect("alpha should be listed in path browser");
    state.path_browse_index = alpha_index;

    handle_key(
        &mut state,
        KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
    )
    .await;
    assert_eq!(state.path_browse_current_dir, expected);
    assert_eq!(state.step, SetupStep::SelectPath);
    assert!(state.path_browse_mode);
}

#[tokio::test]
async fn using_current_folder_returns_to_input_and_requires_second_confirm() {
    let temp = tempfile::tempdir().unwrap();
    let expected = super::tilde_collapse(&temp.path().to_string_lossy());

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

    handle_key(
        &mut state,
        KeyEvent::new(KeyCode::Char('u'), KeyModifiers::NONE),
    )
    .await;

    assert_eq!(state.base_path, expected);
    assert_eq!(state.step, SetupStep::SelectPath);
    assert!(!state.path_browse_mode);

    handle_key(
        &mut state,
        KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
    )
    .await;
    assert_eq!(state.step, SetupStep::Confirm);
}

#[tokio::test]
async fn quick_jumps_and_hidden_toggle_work() {
    let temp = tempfile::tempdir().unwrap();
    let hidden = temp.path().join(".hidden-folder");
    let visible = temp.path().join("visible-folder");
    std::fs::create_dir_all(&hidden).unwrap();
    std::fs::create_dir_all(&visible).unwrap();

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

    assert!(!state.path_browse_show_hidden);
    assert!(state
        .path_browse_entries
        .iter()
        .all(|entry| !entry.label.starts_with(".hidden-folder")));

    handle_key(
        &mut state,
        KeyEvent::new(KeyCode::Char('.'), KeyModifiers::NONE),
    )
    .await;
    assert!(state.path_browse_show_hidden);
    assert!(state
        .path_browse_entries
        .iter()
        .any(|entry| entry.label.starts_with(".hidden-folder")));

    handle_key(
        &mut state,
        KeyEvent::new(KeyCode::Char('.'), KeyModifiers::NONE),
    )
    .await;
    assert!(!state.path_browse_show_hidden);
    assert!(state
        .path_browse_entries
        .iter()
        .all(|entry| !entry.label.starts_with(".hidden-folder")));

    handle_key(
        &mut state,
        KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE),
    )
    .await;
    let cwd = std::env::current_dir().unwrap();
    assert_eq!(
        state.path_browse_current_dir,
        super::tilde_collapse(&cwd.to_string_lossy())
    );

    if let Ok(home) = std::env::var("HOME") {
        handle_key(
            &mut state,
            KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE),
        )
        .await;
        assert_eq!(state.path_browse_current_dir, super::tilde_collapse(&home));
    }

    handle_key(
        &mut state,
        KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE),
    )
    .await;
    let root = cwd.ancestors().last().unwrap();
    assert_eq!(
        state.path_browse_current_dir,
        super::tilde_collapse(&root.to_string_lossy())
    );
}

#[tokio::test]
async fn create_folder_creates_incrementing_names() {
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

    handle_key(
        &mut state,
        KeyEvent::new(KeyCode::Char('n'), KeyModifiers::NONE),
    )
    .await;
    assert!(temp.path().join("new-folder").is_dir());

    handle_key(
        &mut state,
        KeyEvent::new(KeyCode::Char('n'), KeyModifiers::NONE),
    )
    .await;
    assert!(temp.path().join("new-folder-2").is_dir());
    assert!(state
        .path_browse_info
        .as_deref()
        .unwrap_or("")
        .contains("Created"));
}

#[tokio::test]
async fn empty_directory_renders_without_error() {
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
    assert!(state.path_browse_error.is_none());

    let children = state
        .path_browse_entries
        .iter()
        .filter(|entry| entry.label != ".. (parent)")
        .count();
    assert_eq!(children, 0);
}

#[tokio::test]
async fn very_large_directory_list_is_loaded() {
    let temp = tempfile::tempdir().unwrap();
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

    let children: Vec<_> = state
        .path_browse_entries
        .iter()
        .filter(|entry| entry.label.ends_with('/'))
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

    // If current runtime user can still read, skip this check.
    if std::fs::read_dir(&locked).is_ok() {
        let mut reset = std::fs::metadata(&locked).unwrap().permissions();
        reset.set_mode(0o700);
        std::fs::set_permissions(&locked, reset).unwrap();
        return;
    }

    let mut state = SetupState::new(&locked.to_string_lossy());
    state.step = SetupStep::SelectPath;
    state.path_suggestions_mode = false;
    state.base_path = locked.to_string_lossy().to_string();
    state.path_cursor = state.base_path.len();

    handle_key(
        &mut state,
        KeyEvent::new(KeyCode::Char('b'), KeyModifiers::CONTROL),
    )
    .await;
    assert!(state.path_browse_error.is_some());

    let mut reset = std::fs::metadata(&locked).unwrap().permissions();
    reset.set_mode(0o700);
    std::fs::set_permissions(&locked, reset).unwrap();
}
