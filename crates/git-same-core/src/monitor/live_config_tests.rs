use super::*;

fn write(path: &Path, content: &str) {
    std::fs::write(path, content).unwrap();
}

#[test]
fn fixed_config_never_reloads() {
    let live = LiveConfig::new(Config::default(), None);
    assert!(!live.reload_if_changed());
    assert!(live.snapshot().workspaces.is_empty());
}

#[test]
fn unchanged_file_is_not_reloaded() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    write(&path, "workspaces = [\"~/one\"]\n");
    let live = LiveConfig::new(Config::load_from(&path).unwrap(), Some(path));
    assert!(!live.reload_if_changed());
}

#[test]
fn registry_changes_are_picked_up() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    write(&path, "workspaces = [\"~/one\"]\n");
    let live = LiveConfig::new(Config::load_from(&path).unwrap(), Some(path.clone()));

    write(&path, "workspaces = [\"~/one\", \"~/two-longer\"]\n");
    assert!(live.reload_if_changed());
    assert_eq!(live.snapshot().workspaces.len(), 2);

    write(&path, "workspaces = []\n");
    assert!(live.reload_if_changed());
    assert!(live.snapshot().workspaces.is_empty());
}

#[test]
fn file_created_after_start_is_picked_up() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    let live = LiveConfig::new(Config::default(), Some(path.clone()));
    assert!(!live.reload_if_changed());

    write(&path, "[monitor]\nfullscan_interval_secs = 120\n");
    assert!(live.reload_if_changed());
    assert_eq!(live.snapshot().monitor.fullscan_interval_secs, 120);
}

#[test]
fn malformed_reload_keeps_previous_config_and_reports_once() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    write(&path, "workspaces = [\"~/one\"]\n");
    let live = LiveConfig::new(Config::load_from(&path).unwrap(), Some(path.clone()));

    write(&path, "workspaces = [[[ broken and longer\n");
    assert!(!live.reload_if_changed());
    assert_eq!(live.snapshot().workspaces, vec!["~/one".to_string()]);
    assert!(!live.reload_if_changed());
}

#[cfg(unix)]
#[test]
fn transient_read_failure_retries_the_same_file_stamp() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    write(&path, "workspaces = [\"~/one\"]\n");
    let live = LiveConfig::new(Config::load_from(&path).unwrap(), Some(path.clone()));
    write(&path, "workspaces = [\"~/one\", \"~/two\"]\n");
    let permissions = std::fs::metadata(&path).unwrap().permissions();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o000)).unwrap();
    assert!(!live.reload_if_changed());
    std::fs::set_permissions(&path, permissions).unwrap();
    assert!(live.reload_if_changed());
    assert_eq!(live.snapshot().workspaces.len(), 2);
}

/// The caller loads the config, then hands it to `LiveConfig`. A write landing
/// in that gap must not be adopted silently: the snapshot would stay old while
/// the stamp went new, and `reload_if_changed` would never fire again.
#[test]
fn a_write_between_load_and_construction_is_not_lost() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    write(&path, "workspaces = [\"~/one\"]\n");

    // What `prepare_managed` / `run_foreground` do before calling `run_with`.
    let loaded = Config::load_from(&path).unwrap();
    write(&path, "workspaces = [\"~/one\", \"~/two-longer\"]\n");

    let live = LiveConfig::new(loaded, Some(path));
    assert_eq!(
        live.snapshot().workspaces.len(),
        2,
        "the newer file must win, or a just-registered workspace stays invisible"
    );
}

/// Erring the other way is safe: the stamp is taken before the read, so a
/// write racing construction costs at most one redundant reload.
#[test]
fn a_malformed_file_falls_back_to_the_callers_config() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    write(&path, "workspaces = [\"~/one\"]\n");
    let loaded = Config::load_from(&path).unwrap();
    write(&path, "workspaces = [[[ broken\n");

    let live = LiveConfig::new(loaded, Some(path));
    assert_eq!(live.snapshot().workspaces, vec!["~/one".to_string()]);
}
