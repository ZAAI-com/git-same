use super::*;
use std::path::Path;
use std::sync::Mutex;

static HOME_LOCK: Mutex<()> = Mutex::new(());

fn with_temp_home<T>(home: &Path, f: impl FnOnce() -> T) -> T {
    let _lock = HOME_LOCK.lock().expect("HOME lock poisoned");
    let original_home = std::env::var("HOME").ok();
    let original_userprofile = std::env::var("USERPROFILE").ok();

    struct HomeRestore {
        home: Option<String>,
        userprofile: Option<String>,
    }

    impl Drop for HomeRestore {
        fn drop(&mut self) {
            if let Some(value) = self.home.take() {
                std::env::set_var("HOME", value);
            } else {
                std::env::remove_var("HOME");
            }

            if let Some(value) = self.userprofile.take() {
                std::env::set_var("USERPROFILE", value);
            } else {
                std::env::remove_var("USERPROFILE");
            }
        }
    }

    let _restore = HomeRestore {
        home: original_home,
        userprofile: original_userprofile,
    };
    std::env::set_var("HOME", home);
    std::env::set_var("USERPROFILE", home);
    f()
}

#[test]
fn dot_dir_cache_and_config_paths_are_derived_from_root() {
    let root = Path::new("/tmp/my-workspace");

    let dot_dir = WorkspaceStore::dot_dir(root);
    let config = WorkspaceStore::config_path(root);
    let cache = WorkspaceStore::cache_path(root);
    let history = WorkspaceStore::sync_history_path(root);

    assert_eq!(
        dot_dir,
        std::path::PathBuf::from("/tmp/my-workspace/.git-same")
    );
    assert_eq!(
        config,
        std::path::PathBuf::from("/tmp/my-workspace/.git-same/config.toml")
    );
    assert_eq!(
        cache,
        std::path::PathBuf::from("/tmp/my-workspace/.git-same/cache.json")
    );
    assert_eq!(
        history,
        std::path::PathBuf::from("/tmp/my-workspace/.git-same/sync-history.json")
    );
}

#[test]
fn load_returns_error_when_no_config_exists() {
    let temp = tempfile::tempdir().unwrap();
    let err = WorkspaceStore::load(temp.path()).unwrap_err();
    assert!(err.to_string().contains("No workspace config found"));
}

#[test]
fn save_creates_dot_dir_and_config_file() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path().join("home");
    std::fs::create_dir_all(&home).unwrap();
    let root = temp.path().join("my-ws");
    std::fs::create_dir_all(&root).unwrap();

    with_temp_home(&home, || {
        let config_path = crate::config::Config::default_path().unwrap();
        std::fs::create_dir_all(config_path.parent().unwrap()).unwrap();
        std::fs::write(&config_path, crate::config::Config::default_toml()).unwrap();

        let ws = WorkspaceConfig::new_from_root(&root);
        WorkspaceStore::save(&ws).unwrap();

        assert!(WorkspaceStore::config_path(&root).exists());
    });
}

#[test]
fn load_from_path_roundtrip_sets_root_path_from_parent() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("roundtrip");
    let dot_dir = root.join(".git-same");
    std::fs::create_dir_all(&dot_dir).unwrap();

    let ws = WorkspaceConfig::new_from_root(&root);
    let config_path = dot_dir.join("config.toml");
    std::fs::write(&config_path, ws.to_toml().unwrap()).unwrap();

    let loaded = WorkspaceStore::load_from_path(&config_path).unwrap();
    // root_path is canonicalized, so compare the file name component
    assert_eq!(loaded.root_path.file_name(), root.file_name());
}

#[test]
fn delete_returns_error_when_dot_dir_missing() {
    let temp = tempfile::tempdir().unwrap();
    // No .git-same/ directory inside temp, so delete should fail
    let err = WorkspaceStore::delete(temp.path()).unwrap_err();
    assert!(err.to_string().contains("No workspace config found"));
}

#[test]
fn save_returns_error_when_global_config_is_missing() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path().join("home");
    std::fs::create_dir_all(&home).unwrap();

    with_temp_home(&home, || {
        let root = temp.path().join("my-ws");
        std::fs::create_dir_all(&root).unwrap();

        let ws = WorkspaceConfig::new_from_root(&root);
        let err = WorkspaceStore::save(&ws).unwrap_err();
        assert!(err.to_string().contains("Run 'gisa init' first"));
        assert!(!WorkspaceStore::config_path(&root).exists());
        assert!(!WorkspaceStore::dot_dir(&root).exists());
    });
}

#[test]
fn save_updates_registry_when_global_config_exists() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path().join("home");
    std::fs::create_dir_all(&home).unwrap();

    with_temp_home(&home, || {
        let config_path = crate::config::Config::default_path().unwrap();
        std::fs::create_dir_all(config_path.parent().unwrap()).unwrap();
        std::fs::write(&config_path, crate::config::Config::default_toml()).unwrap();

        let root = temp.path().join("my-ws");
        std::fs::create_dir_all(&root).unwrap();

        let ws = WorkspaceConfig::new_from_root(&root);
        WorkspaceStore::save(&ws).unwrap();

        let cfg = crate::config::Config::load_from(&config_path).unwrap();
        assert_eq!(cfg.workspaces.len(), 1);
    });
}

#[test]
fn save_rolls_back_new_workspace_write_when_registry_update_fails() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path().join("home");
    std::fs::create_dir_all(&home).unwrap();

    with_temp_home(&home, || {
        let global_config_path = crate::config::Config::default_path().unwrap();
        std::fs::create_dir_all(global_config_path.parent().unwrap()).unwrap();
        std::fs::write(&global_config_path, "invalid = [").unwrap();

        let root = temp.path().join("my-ws");
        std::fs::create_dir_all(&root).unwrap();

        let ws = WorkspaceConfig::new_from_root(&root);
        let err = WorkspaceStore::save(&ws).unwrap_err();
        assert!(err.to_string().contains("Failed to parse config"));
        assert!(!WorkspaceStore::config_path(&root).exists());
        assert!(!WorkspaceStore::dot_dir(&root).exists());
    });
}

#[test]
fn save_restores_existing_workspace_config_when_registry_update_fails() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path().join("home");
    std::fs::create_dir_all(&home).unwrap();

    with_temp_home(&home, || {
        let global_config_path = crate::config::Config::default_path().unwrap();
        std::fs::create_dir_all(global_config_path.parent().unwrap()).unwrap();
        std::fs::write(&global_config_path, "invalid = [").unwrap();

        let root = temp.path().join("my-ws");
        let dot_dir = WorkspaceStore::dot_dir(&root);
        std::fs::create_dir_all(&dot_dir).unwrap();

        let mut previous = WorkspaceConfig::new_from_root(&root);
        previous.username = "before".to_string();
        let previous_content = previous.to_toml().unwrap();
        let config_path = WorkspaceStore::config_path(&root);
        std::fs::write(&config_path, &previous_content).unwrap();

        let mut ws = WorkspaceConfig::new_from_root(&root);
        ws.username = "after".to_string();
        let err = WorkspaceStore::save(&ws).unwrap_err();
        assert!(err.to_string().contains("Failed to parse config"));

        let restored = std::fs::read_to_string(&config_path).unwrap();
        assert_eq!(restored, previous_content);
        assert!(dot_dir.exists());
    });
}
