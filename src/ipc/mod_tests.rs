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
