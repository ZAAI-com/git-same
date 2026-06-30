use super::*;
use tempfile::TempDir;

#[test]
fn status_response_ends_with_newline() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("status.json");
    let writer = StatusFileWriter::new(path.clone());
    writer
        .write(&FinderStatus::new(0, "2026-06-21T00:00:00Z".to_string()))
        .unwrap();

    let resp = status_response(&path);
    assert!(
        resp.ends_with('\n'),
        "Status response must end with newline"
    );
    assert_ne!(resp, "ERROR\n");
}

#[test]
fn status_response_error_when_missing() {
    let dir = TempDir::new().unwrap();
    let resp = status_response(&dir.path().join("does-not-exist.json"));
    assert_eq!(resp, "ERROR\n");
}
