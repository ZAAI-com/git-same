use super::*;
use crate::output::{Output, Verbosity};

#[test]
fn ensure_base_path_is_noop_when_path_exists() {
    let temp = tempfile::tempdir().unwrap();
    let mut workspace = WorkspaceConfig::new("ws", temp.path().to_string_lossy().to_string());
    let output = Output::new(Verbosity::Quiet, false);

    ensure_base_path(&mut workspace, &output).unwrap();
    assert_eq!(
        workspace.base_path,
        temp.path().to_string_lossy().to_string()
    );
}

#[test]
fn ensure_base_path_rejects_existing_file_path() {
    let temp = tempfile::tempdir().unwrap();
    let file_path = temp.path().join("not-a-directory");
    std::fs::write(&file_path, "x").unwrap();

    let mut workspace = WorkspaceConfig::new("ws", file_path.to_string_lossy().to_string());
    let output = Output::new(Verbosity::Quiet, false);

    let err = ensure_base_path(&mut workspace, &output).unwrap_err();
    assert!(err.to_string().contains("not a directory"));
}

#[test]
fn confirm_stderr_function_signature_is_stable() {
    let _fn_ptr: fn(&str) -> Result<bool> = confirm_stderr;
}
