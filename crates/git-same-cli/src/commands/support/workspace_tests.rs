use super::*;
use git_same_core::output::{Output, Verbosity};

#[test]
fn ensure_base_path_is_noop_when_path_exists() {
    let temp = tempfile::tempdir().unwrap();
    let workspace = WorkspaceConfig::new_from_root(temp.path());
    let output = Output::new(Verbosity::Quiet, false);

    ensure_base_path(&workspace, &output).unwrap();
}

#[test]
fn ensure_base_path_rejects_existing_file_path() {
    let temp = tempfile::tempdir().unwrap();
    let file_path = temp.path().join("not-a-directory");
    std::fs::write(&file_path, "x").unwrap();

    let workspace = WorkspaceConfig::new_from_root(&file_path);
    let output = Output::new(Verbosity::Quiet, false);

    let err = ensure_base_path(&workspace, &output).unwrap_err();
    assert!(err.to_string().contains("not a directory"));
}

#[test]
fn ensure_base_path_errors_on_missing_path() {
    let workspace = WorkspaceConfig::new_from_root(std::path::Path::new("/nonexistent/path/xyz"));
    let output = Output::new(Verbosity::Quiet, false);

    let err = ensure_base_path(&workspace, &output).unwrap_err();
    assert!(err.to_string().contains("does not exist"));
}
