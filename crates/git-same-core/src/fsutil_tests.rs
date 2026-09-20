use super::*;

#[test]
fn atomic_write_creates_parents_and_replaces_content() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nested").join("file.json");
    atomic_write(&path, b"one", None).unwrap();
    atomic_write(&path, b"two", None).unwrap();
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "two");
}

#[test]
fn atomic_write_leaves_no_temp_files() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("file.toml");
    atomic_write(&path, b"x = 1", None).unwrap();
    let names: Vec<_> = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names, vec!["file.toml".to_string()]);
}

#[test]
fn temp_sibling_names_are_unique_and_keep_the_directory() {
    let path = std::path::Path::new("/tmp/example/config.toml");
    let a = temp_sibling(path);
    let b = temp_sibling(path);
    assert_ne!(a, b);
    assert_eq!(a.parent(), path.parent());
    assert!(a.to_string_lossy().ends_with(TEMP_SUFFIX));
}

#[cfg(unix)]
#[test]
fn atomic_write_applies_mode() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("agent.plist");
    atomic_write(&path, b"x", Some(0o644)).unwrap();
    let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o644);
}
