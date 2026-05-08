use super::*;

struct TestDir {
    path: std::path::PathBuf,
}

impl TestDir {
    fn new(name: &str) -> Self {
        let unique = format!(
            "git-same-app-{}-{}-{}",
            name,
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap()
        );
        let path = std::env::temp_dir().join(unique);
        std::fs::create_dir_all(&path).unwrap();
        Self { path }
    }

    fn path(&self) -> &std::path::Path {
        &self.path
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

fn config_with_workspace(root: &std::path::Path) -> Config {
    let mut config = Config::default();
    config.finder.show_ambient = false;
    config.workspaces = vec![root.to_string_lossy().to_string()];
    config
}

fn create_workspace_with_repo(base: &std::path::Path) -> std::path::PathBuf {
    let root = base.join("workspace");
    let repo = root.join("acme/widgets");
    std::fs::create_dir_all(&repo).unwrap();

    let status = std::process::Command::new("git")
        .args(["init", "--quiet"])
        .current_dir(&repo)
        .status()
        .unwrap();
    assert!(status.success());

    let mut workspace = WorkspaceConfig::new_from_root(&root);
    workspace.orgs = vec!["acme".to_string()];
    let dot_dir = root.join(".git-same");
    std::fs::create_dir_all(&dot_dir).unwrap();
    std::fs::write(dot_dir.join("config.toml"), workspace.to_toml().unwrap()).unwrap();

    root
}

#[test]
fn read_status_snapshot_scans_foreground_when_status_file_is_missing() {
    let temp = TestDir::new("missing-status");
    let root = create_workspace_with_repo(temp.path());
    let config = config_with_workspace(&root);
    let ipc = IpcConfig {
        dir: temp.path().join("ipc"),
    };

    let snapshot = read_status_snapshot_with(&config, &ipc).unwrap();

    assert!(snapshot.stale);
    assert!(snapshot.updated_at.is_none());
    let status = snapshot
        .status
        .expect("foreground scan should provide data");
    assert_eq!(status.repos.len(), 1);
    assert!(status.repos[0].path.ends_with("acme/widgets"));
}

#[test]
fn read_status_snapshot_uses_foreground_data_when_monitor_pid_is_stale() {
    let temp = TestDir::new("stale-monitor");
    let root = create_workspace_with_repo(temp.path());
    let config = config_with_workspace(&root);
    let ipc = IpcConfig {
        dir: temp.path().join("ipc"),
    };
    ipc.ensure_dir().unwrap();

    let mut stale_status = FinderStatus::new(u32::MAX, chrono::Utc::now().to_rfc3339());
    stale_status.repos = Vec::new();
    StatusFileWriter::new(ipc.status_file_path())
        .write(&stale_status)
        .unwrap();

    let snapshot = read_status_snapshot_with(&config, &ipc).unwrap();

    assert!(snapshot.stale);
    assert!(snapshot.updated_at.is_some());
    let status = snapshot
        .status
        .expect("stale monitor should fall back to scan");
    assert_eq!(status.repos.len(), 1);
}

#[test]
fn parse_pluginkit_output_marks_enabled_extension() {
    let stdout = "+    com.zaai.git-same.Badges(3.1.0)    \
                  /Applications/Git-Same.app/Contents/PlugIns/GitSameBadges.appex\n";
    let result = parse_pluginkit_output(stdout, FINDER_EXTENSION_ID);
    assert_eq!(
        result,
        ExtensionStatus {
            installed: true,
            enabled: true,
        }
    );
}

#[test]
fn parse_pluginkit_output_marks_disabled_extension() {
    let stdout = "-    com.zaai.git-same.Badges(3.1.0)    \
                  /Applications/Git-Same.app/Contents/PlugIns/GitSameBadges.appex\n";
    let result = parse_pluginkit_output(stdout, FINDER_EXTENSION_ID);
    assert_eq!(
        result,
        ExtensionStatus {
            installed: true,
            enabled: false,
        }
    );
}

#[test]
fn parse_pluginkit_output_returns_uninstalled_for_empty_stdout() {
    let result = parse_pluginkit_output("", FINDER_EXTENSION_ID);
    assert_eq!(
        result,
        ExtensionStatus {
            installed: false,
            enabled: false,
        }
    );
}

#[test]
fn parse_pluginkit_output_ignores_other_extensions() {
    let stdout = "+    com.apple.dt.Xcode.SimulatorTrampoline(15.0)    \
                  /Applications/Xcode.app/Contents/PlugIns/SimulatorTrampoline.appex\n\
                  -    com.example.other(1.0)    /Applications/Other.app\n";
    let result = parse_pluginkit_output(stdout, FINDER_EXTENSION_ID);
    assert_eq!(
        result,
        ExtensionStatus {
            installed: false,
            enabled: false,
        }
    );
}
