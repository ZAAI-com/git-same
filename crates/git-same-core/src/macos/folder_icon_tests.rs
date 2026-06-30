use super::*;
use tempfile::tempdir;

#[test]
fn set_then_clear_round_trip() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();

    // Fresh directory has no custom icon and no Icon\r file.
    assert!(!is_set(root), "fresh dir already has custom icon");

    // Paint the workspace folder icon.
    set(root, WORKSPACE_FOLDER_ICNS).expect("set should succeed");
    assert!(is_set(root), "expected Icon\\r after set");
    let icon_file = root.join("Icon\r");
    assert!(icon_file.exists(), "Icon\\r file missing on disk");

    // Idempotent: setting again should still succeed.
    set(root, WORKSPACE_FOLDER_ICNS).expect("idempotent set should succeed");
    assert!(is_set(root), "Icon\\r should remain after second set");

    // Clear removes the icon.
    clear(root).expect("clear should succeed");
    assert!(!is_set(root), "is_set still true after clear");
    assert!(
        !icon_file.exists(),
        "Icon\\r still on disk after clear: {}",
        icon_file.display()
    );
}

#[test]
fn set_on_nonexistent_path_errors() {
    let dir = tempdir().expect("tempdir");
    let missing = dir.path().join("does-not-exist");
    let err = set(&missing, WORKSPACE_FOLDER_ICNS).expect_err("expected error");
    let msg = err.to_string();
    assert!(
        msg.contains("not a directory"),
        "unexpected error message: {msg}"
    );
}

#[test]
fn clear_on_dir_without_icon_is_noop() {
    let dir = tempdir().expect("tempdir");
    clear(dir.path()).expect("clear should succeed even without prior icon");
    assert!(!is_set(dir.path()));
}
