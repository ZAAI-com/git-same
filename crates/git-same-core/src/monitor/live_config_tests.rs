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
