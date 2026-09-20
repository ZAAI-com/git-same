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
fn present_non_boolean_autostart_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let path = write(&dir, "[monitor]\nautostart = \"yes\"\n");
    let error = read_monitor_autostart(&path).unwrap_err().to_string();
    assert!(error.contains("must be a boolean"));
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

/// `gisa monitor --stop` must work on a machine that has never been
/// configured, without leaving a `config.toml` behind: that would turn the
/// next command's "No configuration found. Run 'gisa init'." into a silently
/// empty workspace list. `launchctl disable` carries the state instead.
#[test]
fn set_autostart_never_creates_a_configuration() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("sub").join("config.toml");

    set_monitor_autostart(&path, false).unwrap();
    assert!(!path.exists());

    set_monitor_autostart(&path, true).unwrap();
    assert!(!path.exists());
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

#[test]
fn stale_settings_form_cannot_undo_a_stop_or_reset_other_sections() {
    let dir = tempfile::tempdir().unwrap();
    let original = "# tuned by hand\nconcurrency = 2\ndefault_workspace = \"~/old\"\n\n[ui]\ncustom_folder_icon = false\n\n[monitor]\n# stopped from the CLI\nautostart = false\nfullscan_interval_secs = 30\n\n[future]\nunknown_key = 1\n";
    let path = write(&dir, original);

    // The form was loaded before the CLI stop, so it knows nothing of it.
    let mut form = Config {
        concurrency: 6,
        default_workspace: None,
        ..Config::default()
    };
    form.monitor.fullscan_interval_secs = 90;
    form.finder.show_ambient = false;
    assert!(form.monitor.autostart, "the form default would re-enable");

    merge_settings(&path, &form).unwrap();

    let saved = Config::load_from(&path).unwrap();
    assert_eq!(saved.concurrency, 6);
    assert_eq!(saved.default_workspace, None);
    assert_eq!(saved.monitor.fullscan_interval_secs, 90);
    assert!(!saved.monitor.autostart, "CLI stop must survive");
    assert!(!saved.ui.custom_folder_icon, "[ui] must survive");
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains("# tuned by hand"));
    assert!(text.contains("# stopped from the CLI"));
    assert!(text.contains("unknown_key = 1"));
}

#[test]
fn merge_settings_refuses_a_malformed_file() {
    let dir = tempfile::tempdir().unwrap();
    let broken = "concurrency = = 3\n";
    let path = write(&dir, broken);
    assert!(merge_settings(&path, &Config::default()).is_err());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), broken);
}

#[test]
fn merge_settings_round_trips_every_modeled_field() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    let mut form = Config {
        structure: "{provider}/{org}/{repo}".to_string(),
        workspaces: vec!["~/a".to_string(), "~/b".to_string()],
        ..Config::default()
    };
    form.finder.scan_roots = vec!["~/code".to_string()];
    form.filters.include_archived = !form.filters.include_archived;

    merge_settings(&path, &form).unwrap();

    let saved = Config::load_from(&path).unwrap();
    assert_eq!(saved.structure, form.structure);
    assert_eq!(saved.workspaces, form.workspaces);
    assert_eq!(saved.finder.scan_roots, form.finder.scan_roots);
    assert_eq!(
        saved.filters.include_archived,
        form.filters.include_archived
    );
}

// ------------------------------------------- registry and default workspace
//
// `gisa setup`, `gisa scan --register`, `gisa workspace default` and the app's
// Save button used to re-serialize the whole document (dropping every comment)
// or to edit it as raw text (inserting `default_workspace` into whichever
// table happened to follow the first textual `sync_mode`).

/// A comment that mentions `sync_mode` from inside another table: the old
/// text-surgery writer inserted `default_workspace` into `[filters]` here,
/// where `Config` silently ignores it.
const ANNOTATED: &str = r#"# git-same configuration
structure = "{org}/{repo}"

# How existing repos are updated.
sync_mode = "fetch"

[filters]
# sync_mode can be overridden per workspace one day.
include_forks = false
"#;

#[test]
fn setting_a_default_workspace_keeps_comments_and_table_structure() {
    let dir = tempfile::tempdir().unwrap();
    let path = write(&dir, ANNOTATED);

    Config::save_default_workspace_to(&path, Some("~/work")).unwrap();

    let content = std::fs::read_to_string(&path).unwrap();
    assert!(content.contains("# git-same configuration"), "{content}");
    assert!(
        content.contains("# How existing repos are updated."),
        "{content}"
    );
    assert!(
        content.contains("# sync_mode can be overridden per workspace one day."),
        "{content}"
    );
    let config = Config::load_from(&path).unwrap();
    assert_eq!(config.default_workspace.as_deref(), Some("~/work"));
    assert!(!config.filters.include_forks);
}

#[test]
fn clearing_the_default_workspace_removes_only_that_key() {
    let dir = tempfile::tempdir().unwrap();
    let path = write(&dir, ANNOTATED);
    Config::save_default_workspace_to(&path, Some("~/work")).unwrap();

    Config::save_default_workspace_to(&path, None).unwrap();

    let content = std::fs::read_to_string(&path).unwrap();
    assert!(!content.contains("default_workspace"), "{content}");
    assert!(content.contains("# git-same configuration"), "{content}");
    assert_eq!(Config::load_from(&path).unwrap().default_workspace, None);
}

#[test]
fn registering_and_unregistering_a_workspace_keeps_comments() {
    let dir = tempfile::tempdir().unwrap();
    let path = write(&dir, &Config::default_toml());
    let before = std::fs::read_to_string(&path).unwrap();
    let comments = before
        .lines()
        .filter(|l| l.trim_start().starts_with('#'))
        .count();
    assert!(comments > 5, "the default config is documented");

    Config::add_to_registry_at(&path, "~/one").unwrap();
    Config::add_to_registry_at(&path, "~/two").unwrap();
    // Registering the same path twice is idempotent.
    Config::add_to_registry_at(&path, "~/two").unwrap();
    Config::remove_from_registry_at(&path, "~/one").unwrap();

    let after = std::fs::read_to_string(&path).unwrap();
    assert_eq!(
        after
            .lines()
            .filter(|l| l.trim_start().starts_with('#'))
            .count(),
        comments,
        "every comment must survive:\n{after}"
    );
    assert_eq!(
        Config::load_from(&path).unwrap().workspaces,
        vec!["~/two".to_string()]
    );
}

#[test]
fn a_malformed_config_is_not_rewritten_by_a_registry_change() {
    let dir = tempfile::tempdir().unwrap();
    let broken = "workspaces = [[[\n";
    let path = write(&dir, broken);

    assert!(Config::add_to_registry_at(&path, "~/one").is_err());
    assert!(Config::save_default_workspace_to(&path, Some("~/one")).is_err());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), broken);
}
