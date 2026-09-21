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
fn atomic_write_accepts_a_bare_relative_filename() {
    static CWD_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _lock = CWD_LOCK.lock().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let previous = std::env::current_dir().unwrap();
    std::env::set_current_dir(dir.path()).unwrap();
    let result = atomic_write(Path::new("file.json"), b"one", None);
    std::env::set_current_dir(previous).unwrap();
    result.unwrap();
    assert_eq!(std::fs::read(dir.path().join("file.json")).unwrap(), b"one");
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

#[cfg(unix)]
#[test]
fn atomic_write_preserves_existing_mode_when_unspecified() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    std::fs::write(&path, "old").unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
    atomic_write(&path, b"new", None).unwrap();
    assert_eq!(
        std::fs::metadata(path).unwrap().permissions().mode() & 0o777,
        0o600
    );
}

#[cfg(unix)]
#[test]
fn atomic_write_preserves_relative_symlink() {
    use std::os::unix::fs::symlink;
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("real.toml");
    let link = dir.path().join("config.toml");
    std::fs::write(&target, "old").unwrap();
    symlink("real.toml", &link).unwrap();
    atomic_write(&link, b"new", None).unwrap();
    assert!(std::fs::symlink_metadata(&link)
        .unwrap()
        .file_type()
        .is_symlink());
    assert_eq!(std::fs::read_to_string(target).unwrap(), "new");
}

#[cfg(unix)]
#[test]
fn atomic_write_preserves_absolute_symlink() {
    use std::os::unix::fs::symlink;
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("real.toml");
    let link = dir.path().join("config.toml");
    std::fs::write(&target, "old").unwrap();
    symlink(&target, &link).unwrap();
    atomic_write(&link, b"new", None).unwrap();
    assert!(std::fs::symlink_metadata(&link)
        .unwrap()
        .file_type()
        .is_symlink());
    assert_eq!(std::fs::read_to_string(target).unwrap(), "new");
}

#[test]
fn sync_dir_ignores_missing_directories() {
    sync_dir(Some(std::path::Path::new(
        "/nonexistent-directory-for-tests",
    )));
    sync_dir(None);
}
