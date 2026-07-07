use super::*;

#[test]
fn test_ipc_config_paths() {
    let config = IpcConfig {
        dir: PathBuf::from("/home/user/.config/git-same/finder"),
    };
    assert_eq!(
        config.status_file_path(),
        PathBuf::from("/home/user/.config/git-same/finder/status.json")
    );
    assert_eq!(
        config.preferences_path(),
        PathBuf::from("/home/user/.config/git-same/finder/preferences.json")
    );
}

#[cfg(unix)]
#[test]
fn test_ipc_config_socket_path() {
    let config = IpcConfig {
        dir: PathBuf::from("/home/user/.config/git-same/finder"),
    };
    assert_eq!(
        config.socket_path(),
        PathBuf::from("/home/user/.config/git-same/finder/finder.sock")
    );
}

#[test]
fn test_ensure_dir_creates_directory() {
    let temp = tempfile::tempdir().unwrap();
    let config = IpcConfig {
        dir: temp.path().join("finder"),
    };
    assert!(!config.dir.exists());
    config.ensure_dir().unwrap();
    assert!(config.dir.exists());
}

#[test]
fn test_app_group_id_has_team_prefix() {
    // Apple requires the team-id prefix on app-group identifiers; this guard
    // catches accidental edits to the constant.
    assert!(APP_GROUP_ID.starts_with("group.57KL6Y7V32."));
    assert!(APP_GROUP_ID.ends_with(".com.zaai.git-same"));
}

#[cfg(target_os = "macos")]
#[test]
fn test_macos_group_container_dir_includes_app_group_segment() {
    // We don't mutate HOME (env mutation races with parallel tests); instead
    // we just assert that, when HOME is set in the inherited environment, the
    // function returns a path under Library/Group Containers/<APP_GROUP_ID>.
    if let Some(dir) = macos_group_container_dir() {
        let dir_str = dir.to_string_lossy();
        assert!(
            dir_str.contains("/Library/Group Containers/"),
            "expected Library/Group Containers/ in path, got {}",
            dir_str
        );
        assert!(
            dir_str.ends_with(APP_GROUP_ID),
            "expected to end with {}, got {}",
            APP_GROUP_ID,
            dir_str
        );
    }
}

#[cfg(target_os = "macos")]
#[test]
fn test_default_path_uses_group_container_on_macos() {
    if std::env::var_os("HOME").is_none() {
        return;
    }
    let cfg = IpcConfig::default_path().expect("default_path");
    assert!(
        cfg.dir
            .ends_with("Library/Group Containers/group.57KL6Y7V32.com.zaai.git-same"),
        "expected group-container suffix, got {}",
        cfg.dir.display()
    );
}

// On Windows, directories::ProjectDirs adds a `config` component
// (`git-same\config`), so the legacy layout only ends in `git-same/finder` on
// Unix (macOS/Linux), matching the sibling macOS-gated default-path test.
#[cfg(unix)]
#[test]
fn test_legacy_default_path_ends_in_finder() {
    // legacy_default_path leans on Config::default_path which respects XDG
    // env vars; we just sanity-check the suffix.
    if let Ok(cfg) = IpcConfig::legacy_default_path() {
        assert!(
            cfg.dir.ends_with("git-same/finder"),
            "expected 'git-same/finder' suffix, got {}",
            cfg.dir.display()
        );
    }
}

#[test]
fn test_host_status_path_matches_legacy_default_path() {
    // The host reads from the non-container host path; it must resolve to the
    // same directory as legacy_default_path (a distinct name for clarity).
    let host = IpcConfig::host_status_path();
    let legacy = IpcConfig::legacy_default_path();
    match (host, legacy) {
        (Ok(host), Ok(legacy)) => {
            assert_eq!(host.dir, legacy.dir);
            // On Windows the dir ends in `git-same\config\finder` (see the
            // comment on test_legacy_default_path_ends_in_finder), so the
            // suffix check is unix-only; the equality above is the real point.
            #[cfg(unix)]
            assert!(host.dir.ends_with("git-same/finder"));
        }
        (Err(_), Err(_)) => {}
        _ => panic!("host_status_path and legacy_default_path disagreed on success"),
    }
}

#[test]
fn test_status_writer_has_no_mirrors_for_custom_dir() {
    // A caller-supplied dir (tests, embedders) must never leak mirror writes
    // into the real user's host dir.
    let temp = tempfile::tempdir().unwrap();
    let config = IpcConfig {
        dir: temp.path().join("ipc"),
    };
    let writer = config.status_writer();
    assert_eq!(writer.path(), config.status_file_path().as_path());
    assert!(writer.mirror_paths().is_empty());
}

#[cfg(target_os = "macos")]
#[test]
fn test_status_writer_mirrors_host_status_for_group_container() {
    if std::env::var_os("HOME").is_none() {
        return;
    }
    let config = IpcConfig::default_path().expect("default_path");
    let writer = config.status_writer();
    if Some(config.dir.as_path()) == macos_group_container_dir().as_deref() {
        let host = IpcConfig::host_status_path().expect("host_status_path");
        assert_eq!(
            writer.mirror_paths().to_vec(),
            vec![host.status_file_path()]
        );
    } else {
        // Legacy fallback (HOME unset is handled above; this arm is defensive).
        assert!(writer.mirror_paths().is_empty());
    }
}
