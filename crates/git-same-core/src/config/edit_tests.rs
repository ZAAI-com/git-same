use super::*;

fn write(dir: &tempfile::TempDir, content: &str) -> std::path::PathBuf {
    let path = dir.path().join("config.toml");
    std::fs::write(&path, content).unwrap();
    path
}

#[test]
fn autostart_defaults_to_true_when_file_or_key_is_missing() {
    let dir = tempfile::tempdir().unwrap();
    assert!(read_monitor_autostart(&dir.path().join("missing.toml")).unwrap());
    let path = write(&dir, "[monitor]\nfullscan_interval_secs = 60\n");
    assert!(read_monitor_autostart(&path).unwrap());
}

#[test]
fn explicit_false_is_read_back() {
    let dir = tempfile::tempdir().unwrap();
    let path = write(&dir, "[monitor]\nautostart = false\n");
    assert!(!read_monitor_autostart(&path).unwrap());
}

#[test]
fn set_autostart_preserves_comments_and_unrelated_keys() {
    let dir = tempfile::tempdir().unwrap();
    let original = "# my notes\nconcurrency = 7 # tuned\n\n[ui]\ncustom_folder_icon = false\n\n[monitor]\n# cadence\nfullscan_interval_secs = 45\n";
    let path = write(&dir, original);

    set_monitor_autostart(&path, false).unwrap();

    let updated = std::fs::read_to_string(&path).unwrap();
    assert!(updated.contains("# my notes"));
    assert!(updated.contains("concurrency = 7 # tuned"));
    assert!(updated.contains("custom_folder_icon = false"));
    assert!(updated.contains("# cadence"));
    assert!(updated.contains("fullscan_interval_secs = 45"));
    assert!(updated.contains("autostart = false"));
    let config = Config::load_from(&path).unwrap();
    assert!(!config.monitor.autostart);
    assert_eq!(config.concurrency, 7);
}

#[test]
fn set_autostart_adds_monitor_table_when_absent() {
    let dir = tempfile::tempdir().unwrap();
    let path = write(&dir, "concurrency = 2\n");
    set_monitor_autostart(&path, false).unwrap();
    assert!(!read_monitor_autostart(&path).unwrap());
    assert_eq!(Config::load_from(&path).unwrap().concurrency, 2);
}

#[test]
fn set_autostart_creates_default_config_only_when_missing() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("sub").join("config.toml");
    set_monitor_autostart(&path, false).unwrap();
    let config = Config::load_from(&path).unwrap();
    assert!(!config.monitor.autostart);
    assert_eq!(config.monitor.fullscan_interval_secs, 30);
}

#[test]
fn malformed_config_is_left_byte_identical() {
    let dir = tempfile::tempdir().unwrap();
    let broken = "concurrency = = 3\n[monitor\n";
    let path = write(&dir, broken);

    assert!(set_monitor_autostart(&path, false).is_err());
    assert!(read_monitor_autostart(&path).is_err());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), broken);
}

#[test]
fn non_table_monitor_key_is_rejected_without_writing() {
    let dir = tempfile::tempdir().unwrap();
    let content = "monitor = 3\n";
    let path = write(&dir, content);
    assert!(set_monitor_autostart(&path, true).is_err());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), content);
}
