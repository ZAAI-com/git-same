use super::*;
use std::path::Path;
use std::sync::Mutex;

static HOME_LOCK: Mutex<()> = Mutex::new(());

fn with_temp_home<T>(home: &Path, f: impl FnOnce() -> T) -> T {
    let _lock = HOME_LOCK.lock().expect("HOME lock poisoned");
    let original_home = std::env::var("HOME").ok();

    std::env::set_var("HOME", home);
    let result = f();

    if let Some(value) = original_home {
        std::env::set_var("HOME", value);
    } else {
        std::env::remove_var("HOME");
    }

    result
}

#[test]
fn workspace_and_cache_paths_are_derived_from_workspace_name() {
    let temp = tempfile::tempdir().unwrap();

    with_temp_home(temp.path(), || {
        let workspace_dir = WorkspaceStore::workspace_dir("alpha").unwrap();
        let cache_path = WorkspaceStore::cache_path("alpha").unwrap();

        assert_eq!(workspace_dir, temp.path().join(".config/git-same/alpha"));
        assert_eq!(
            cache_path,
            temp.path()
                .join(".config/git-same/alpha/workspace-cache.json")
        );
    });
}

#[test]
fn load_from_path_roundtrip_sets_name_from_parent_directory() {
    let temp = tempfile::tempdir().unwrap();
    let workspace_dir = temp.path().join("roundtrip");
    std::fs::create_dir_all(&workspace_dir).unwrap();

    let config_path = workspace_dir.join("workspace-config.toml");
    let workspace = WorkspaceConfig::new("ignored-name", "/tmp/roundtrip");
    std::fs::write(&config_path, workspace.to_toml().unwrap()).unwrap();

    let loaded = WorkspaceStore::load_from_path(&config_path).unwrap();
    assert_eq!(loaded.name, "roundtrip");
    assert_eq!(loaded.base_path, "/tmp/roundtrip");
}

#[test]
fn save_load_and_list_roundtrip_in_empty_config_root() {
    let temp = tempfile::tempdir().unwrap();

    with_temp_home(temp.path(), || {
        let listed_before = WorkspaceStore::list().unwrap();
        assert!(listed_before.is_empty());

        let workspace = WorkspaceConfig::new("team-alpha", "/tmp/team-alpha");
        WorkspaceStore::save(&workspace).unwrap();

        let loaded = WorkspaceStore::load("team-alpha").unwrap();
        assert_eq!(loaded.name, "team-alpha");
        assert_eq!(loaded.base_path, "/tmp/team-alpha");

        let listed_after = WorkspaceStore::list().unwrap();
        assert_eq!(listed_after.len(), 1);
        assert_eq!(listed_after[0].name, "team-alpha");
    });
}

#[test]
fn delete_nonexistent_workspace_returns_error() {
    let temp = tempfile::tempdir().unwrap();

    with_temp_home(temp.path(), || {
        let err = WorkspaceStore::delete("ghost-workspace").unwrap_err();
        assert!(err.to_string().contains("not found"));
    });
}

#[test]
fn workspace_name_rejects_path_traversal() {
    let temp = tempfile::tempdir().unwrap();

    with_temp_home(temp.path(), || {
        let err = WorkspaceStore::workspace_dir("../escape").unwrap_err();
        assert!(err.to_string().contains("Invalid workspace name"));
    });
}

#[test]
fn workspace_name_allows_safe_characters() {
    let temp = tempfile::tempdir().unwrap();

    with_temp_home(temp.path(), || {
        let path = WorkspaceStore::workspace_dir("team.alpha-1_repo").unwrap();
        assert_eq!(path, temp.path().join(".config/git-same/team.alpha-1_repo"));
    });
}
