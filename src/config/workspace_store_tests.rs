use super::*;
use std::path::Path;
use std::sync::Mutex;

static HOME_LOCK: Mutex<()> = Mutex::new(());

fn with_temp_home<T>(home: &Path, f: impl FnOnce() -> T) -> T {
    let _lock = HOME_LOCK.lock().expect("HOME lock poisoned");
    let original_home = std::env::var("HOME").ok();
    let original_userprofile = std::env::var("USERPROFILE").ok();
    let original_xdg_config_home = std::env::var("XDG_CONFIG_HOME").ok();
    #[cfg(windows)]
    let original_appdata = std::env::var("APPDATA").ok();

    struct HomeRestore {
        home: Option<String>,
        userprofile: Option<String>,
        xdg_config_home: Option<String>,
        #[cfg(windows)]
        appdata: Option<String>,
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

            if let Some(value) = self.xdg_config_home.take() {
                std::env::set_var("XDG_CONFIG_HOME", value);
            } else {
                std::env::remove_var("XDG_CONFIG_HOME");
            }

            #[cfg(windows)]
            if let Some(value) = self.appdata.take() {
                std::env::set_var("APPDATA", value);
            } else {
                std::env::remove_var("APPDATA");
            }
        }
    }

    let _restore = HomeRestore {
        home: original_home,
        userprofile: original_userprofile,
        xdg_config_home: original_xdg_config_home,
        #[cfg(windows)]
        appdata: original_appdata,
    };
    std::env::set_var("HOME", home);
    std::env::set_var("USERPROFILE", home);
    std::env::set_var("XDG_CONFIG_HOME", home.join(".config"));
    #[cfg(windows)]
    {
        let appdata = home.join("AppData").join("Roaming");
        std::fs::create_dir_all(&appdata).ok();
        std::env::set_var("APPDATA", &appdata);
    }
    f()
}

#[test]
fn dot_dir_cache_and_config_paths_are_derived_from_root() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("my-workspace");
    std::fs::create_dir_all(&root).unwrap();

    let dot_dir = WorkspaceStore::dot_dir(&root);
    let config = WorkspaceStore::config_path(&root);
    let cache = WorkspaceStore::cache_path(&root);
    let history = WorkspaceStore::sync_history_path(&root);

    assert_eq!(dot_dir, root.join(".git-same"));
    assert_eq!(config, root.join(".git-same/config.toml"));
    assert_eq!(cache, root.join(".git-same/cache.json"));
    assert_eq!(history, root.join(".git-same/sync-history.json"));
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
fn delete_keeps_workspace_files_when_registry_update_fails() {
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
        std::fs::write(
            dot_dir.join("config.toml"),
            "[provider]\nkind = \"github\"\n",
        )
        .unwrap();

        let err = WorkspaceStore::delete(&root).unwrap_err();
        assert!(err.to_string().contains("Failed to parse config"));
        assert!(
            dot_dir.exists(),
            ".git-same should remain when unregister fails"
        );
    });
}

#[test]
fn delete_with_relative_root_removes_registered_workspace() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path().join("home");
    std::fs::create_dir_all(&home).unwrap();

    with_temp_home(&home, || {
        let global_config_path = crate::config::Config::default_path().unwrap();
        std::fs::create_dir_all(global_config_path.parent().unwrap()).unwrap();
        std::fs::write(&global_config_path, crate::config::Config::default_toml()).unwrap();

        let root = temp.path().join("my-ws");
        let dot_dir = WorkspaceStore::dot_dir(&root);
        std::fs::create_dir_all(&dot_dir).unwrap();
        std::fs::write(
            dot_dir.join("config.toml"),
            "[provider]\nkind = \"github\"\n",
        )
        .unwrap();

        let canonical_root = std::fs::canonicalize(&root).unwrap();
        let registry_path = crate::config::workspace::tilde_collapse_path(&canonical_root);
        crate::config::Config::add_to_registry_at(&global_config_path, &registry_path).unwrap();

        let original_cwd = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp.path()).unwrap();
        struct CwdRestore(std::path::PathBuf);
        impl Drop for CwdRestore {
            fn drop(&mut self) {
                let _ = std::env::set_current_dir(&self.0);
            }
        }
        let _cwd_restore = CwdRestore(original_cwd);

        WorkspaceStore::delete(std::path::Path::new("my-ws")).unwrap();

        assert!(!dot_dir.exists(), ".git-same should be deleted");
        let cfg = crate::config::Config::load_from(&global_config_path).unwrap();
        assert!(
            cfg.workspaces.is_empty(),
            "workspace registry should be empty after delete"
        );
    });
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
fn save_with_registry_config_path_uses_explicit_config_file() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("my-ws");
    std::fs::create_dir_all(&root).unwrap();

    let custom_config_path = temp.path().join("custom-config.toml");
    std::fs::write(&custom_config_path, crate::config::Config::default_toml()).unwrap();

    let ws = WorkspaceConfig::new_from_root(&root);
    WorkspaceStore::save_with_registry_config_path(&ws, &custom_config_path).unwrap();

    assert!(WorkspaceStore::config_path(&root).exists());

    let cfg = crate::config::Config::load_from(&custom_config_path).unwrap();
    assert_eq!(cfg.workspaces.len(), 1);
    assert_eq!(
        cfg.workspaces[0],
        crate::config::workspace::tilde_collapse_path(&root)
    );
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
