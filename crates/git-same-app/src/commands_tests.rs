use super::*;
use std::sync::{Mutex, MutexGuard};

static CONFIG_ENV_LOCK: Mutex<()> = Mutex::new(());

struct ConfigEnvGuard {
    _lock: MutexGuard<'static, ()>,
    previous: Option<String>,
}

impl ConfigEnvGuard {
    fn new(path: &std::path::Path) -> Self {
        let lock = CONFIG_ENV_LOCK.lock().unwrap();
        let previous = std::env::var("GIT_SAME_CONFIG_DIR").ok();
        std::env::set_var("GIT_SAME_CONFIG_DIR", path);
        Self {
            _lock: lock,
            previous,
        }
    }
}

impl Drop for ConfigEnvGuard {
    fn drop(&mut self) {
        match &self.previous {
            Some(previous) => std::env::set_var("GIT_SAME_CONFIG_DIR", previous),
            None => std::env::remove_var("GIT_SAME_CONFIG_DIR"),
        }
    }
}

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

fn default_provider_input() -> WorkspaceProviderDto {
    WorkspaceProviderDto {
        kind: "github".to_string(),
        label: "GitHub".to_string(),
        api_url: None,
        prefer_ssh: true,
    }
}

fn default_filter_input() -> FilterOptionsDto {
    FilterOptionsDto {
        include_archived: false,
        include_forks: false,
        orgs: Vec::new(),
        exclude_repos: Vec::new(),
    }
}

fn default_clone_input() -> CloneOptionsDto {
    CloneOptionsDto {
        depth: 0,
        branch: String::new(),
        recurse_submodules: false,
    }
}

fn workspace_input(root: &std::path::Path) -> WorkspaceInput {
    WorkspaceInput {
        id: None,
        root: root.display().to_string(),
        provider: default_provider_input(),
        username: "manuel".to_string(),
        orgs: vec!["acme".to_string()],
        include_repos: vec!["acme/widgets".to_string()],
        exclude_repos: vec!["acme/legacy".to_string()],
        structure: Some("{org}/{repo}".to_string()),
        sync_mode: Some("fetch".to_string()),
        clone_options: Some(default_clone_input()),
        filters: default_filter_input(),
        concurrency: Some(2),
        refresh_interval: Some(20),
        default: true,
    }
}

#[test]
fn render_monitor_plist_replaces_binary_placeholder() {
    let temp = TestDir::new("monitor-plist");
    let binary = temp.path().join("git-same");
    std::fs::write(&binary, "#!/bin/sh\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = std::fs::metadata(&binary).unwrap().permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&binary, permissions).unwrap();
    }

    let rendered = render_monitor_plist(&binary).unwrap();

    assert!(rendered.contains(&binary.display().to_string()));
    assert!(!rendered.contains("__GIT_SAME_MONITOR_BINARY__"));
    assert!(rendered.contains("com.zaai.git-same.monitor"));
}

// Executability is a Unix permission concept; is_executable() treats every
// existing file as runnable on non-Unix, so this rejection only applies there.
#[cfg(unix)]
#[test]
fn render_monitor_plist_rejects_non_executable_binary() {
    let temp = TestDir::new("monitor-plist-invalid");
    let binary = temp.path().join("git-same");
    std::fs::write(&binary, "").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = std::fs::metadata(&binary).unwrap().permissions();
        permissions.set_mode(0o644);
        std::fs::set_permissions(&binary, permissions).unwrap();
    }

    let error = render_monitor_plist(&binary).unwrap_err().to_string();

    assert!(error.contains("not executable"));
}

#[test]
fn monitor_requirement_message_distinguishes_missing_plist() {
    let agent = MonitorLaunchAgentStatusDto {
        label: MONITOR_LAUNCH_AGENT_LABEL.to_string(),
        plist_path: "/tmp/missing.plist".to_string(),
        binary_path: None,
        installed: false,
        loaded: false,
        running: false,
        state: "missing_plist".to_string(),
        message: "LaunchAgent plist is missing".to_string(),
    };

    assert_eq!(
        monitor_requirement_message(Some(&agent), None, "3.2.0"),
        "LaunchAgent plist missing"
    );
    assert_eq!(
        monitor_requirement_suggestion(Some(&agent), None, "3.2.0"),
        Some("Install the Git-Same monitor LaunchAgent".to_string())
    );
}

fn running_agent() -> MonitorLaunchAgentStatusDto {
    MonitorLaunchAgentStatusDto {
        label: MONITOR_LAUNCH_AGENT_LABEL.to_string(),
        plist_path: "/tmp/agent.plist".to_string(),
        binary_path: Some("/usr/local/bin/git-same".to_string()),
        installed: true,
        loaded: true,
        running: true,
        state: "running".to_string(),
        message: "Monitor running".to_string(),
    }
}

fn snapshot_with_monitor_version(version: Option<&str>) -> StatusSnapshot {
    let mut status = FinderStatus::new(4242, "2026-07-07T00:00:00Z".to_string());
    status.monitor_version = version.map(str::to_string);
    StatusSnapshot {
        status_path: "/tmp/status.json".to_string(),
        updated_at: Some("2026-07-07T00:00:00Z".to_string()),
        stale: false,
        status: Some(status),
    }
}

#[test]
fn monitor_requirement_flags_version_skew() {
    let agent = running_agent();
    let snapshot = snapshot_with_monitor_version(Some("3.1.0"));

    assert_eq!(
        monitor_requirement_message(Some(&agent), Some(&snapshot), "3.2.0"),
        "Monitor is running a different build (3.1.0) than the app (3.2.0)"
    );
    assert_eq!(
        monitor_requirement_suggestion(Some(&agent), Some(&snapshot), "3.2.0"),
        Some("Restart the monitor so it runs the same build as the app".to_string())
    );
}

#[test]
fn monitor_requirement_fails_pass_on_version_skew() {
    let agent = running_agent();

    // A running monitor on a mismatched build must not pass, so the row's
    // state agrees with its "different build" message and restart suggestion.
    let skewed = snapshot_with_monitor_version(Some("3.1.0"));
    assert!(!monitor_requirement_passed(
        Some(&agent),
        Some(&skewed),
        "3.2.0"
    ));

    // Matching builds still pass.
    let matched = snapshot_with_monitor_version(Some("3.2.0"));
    assert!(monitor_requirement_passed(
        Some(&agent),
        Some(&matched),
        "3.2.0"
    ));
}

#[test]
fn monitor_requirement_ignores_matching_version() {
    let agent = running_agent();
    let snapshot = snapshot_with_monitor_version(Some("3.2.0"));

    // Matching versions surface the healthy updated_at message and no skew hint.
    assert_eq!(
        monitor_requirement_message(Some(&agent), Some(&snapshot), "3.2.0"),
        "2026-07-07T00:00:00Z"
    );
    assert_eq!(
        monitor_requirement_suggestion(Some(&agent), Some(&snapshot), "3.2.0"),
        None
    );
}

#[test]
fn read_status_snapshot_returns_none_when_status_file_is_missing() {
    let temp = TestDir::new("missing-status");
    let ipc = IpcConfig {
        dir: temp.path().join("ipc"),
    };

    let snapshot = read_status_snapshot_with(&ipc).unwrap();

    assert!(snapshot.stale);
    assert!(snapshot.updated_at.is_none());
    assert!(
        snapshot.status.is_none(),
        "missing status file must not trigger a fallback scan"
    );
}

#[test]
fn read_status_snapshot_returns_last_known_status_when_monitor_pid_is_stale() {
    let temp = TestDir::new("stale-monitor");
    let ipc = IpcConfig {
        dir: temp.path().join("ipc"),
    };
    ipc.ensure_dir().unwrap();

    let mut stale_status = FinderStatus::new(u32::MAX, chrono::Utc::now().to_rfc3339());
    stale_status.repos = Vec::new();
    StatusFileWriter::new(ipc.status_file_path())
        .write(&stale_status)
        .unwrap();

    let snapshot = read_status_snapshot_with(&ipc).unwrap();

    assert!(snapshot.stale);
    assert!(snapshot.updated_at.is_some());
    let status = snapshot
        .status
        .expect("stale monitor should still surface the last-known status from disk");
    assert!(status.repos.is_empty());
}

#[cfg(unix)]
#[test]
fn read_status_snapshot_removes_a_status_symlink_and_reports_absent() {
    use std::os::unix::fs::symlink;

    let temp = TestDir::new("status-symlink");
    let ipc = IpcConfig {
        dir: temp.path().join("ipc"),
    };
    ipc.ensure_dir().unwrap();

    // Simulate the pre-upgrade layout: status.json is a symlink into another
    // location (the app-group container). Following it would re-trigger the
    // cross-app TCC prompt.
    let external_target = temp.path().join("container-status.json");
    let mut external = FinderStatus::new(4242, chrono::Utc::now().to_rfc3339());
    external.repos = Vec::new();
    StatusFileWriter::new(external_target.clone())
        .write(&external)
        .unwrap();
    let status_path = ipc.status_file_path();
    symlink(&external_target, &status_path).unwrap();
    assert!(std::fs::symlink_metadata(&status_path)
        .unwrap()
        .file_type()
        .is_symlink());

    let snapshot = read_status_snapshot_with(&ipc).unwrap();

    // The guard unlinks the symlink and reports status absent rather than
    // dereferencing it into the container.
    assert!(snapshot.status.is_none());
    assert!(snapshot.stale);
    assert!(
        std::fs::symlink_metadata(&status_path).is_err(),
        "status.json symlink must be removed"
    );
}

#[test]
fn read_status_snapshot_reports_stale_when_status_file_is_corrupt() {
    let temp = TestDir::new("status-corrupt");
    let ipc = IpcConfig {
        dir: temp.path().join("ipc"),
    };
    ipc.ensure_dir().unwrap();
    std::fs::write(ipc.status_file_path(), "{ not json").unwrap();

    let snapshot = read_status_snapshot_with(&ipc).unwrap();

    // A corrupt file must degrade to "no status, stale", not an error.
    assert!(snapshot.status.is_none());
    assert!(snapshot.stale);
    assert!(snapshot.updated_at.is_some());
}

#[test]
fn ensure_config_creates_default_config() {
    let temp = TestDir::new("ensure-config");
    let _env = ConfigEnvGuard::new(temp.path());

    let config = ensure_config().unwrap();

    assert!(config.exists);
    assert!(std::path::Path::new(&config.config_path).exists());
    assert_eq!(config.sync_mode, "fetch");
    assert_eq!(config.structure, "{org}/{repo}");
}

#[test]
fn save_app_config_round_trips_structured_fields() {
    let temp = TestDir::new("save-config");
    let _env = ConfigEnvGuard::new(temp.path());
    ensure_config().unwrap();

    let saved = save_app_config(AppConfigInput {
        structure: "{provider}/{org}/{repo}".to_string(),
        concurrency: 3,
        sync_mode: "pull".to_string(),
        default_workspace: Some("~/repos".to_string()),
        refresh_interval: 60,
        clone: CloneOptionsDto {
            depth: 1,
            branch: "main".to_string(),
            recurse_submodules: true,
        },
        filters: FilterOptionsDto {
            include_archived: true,
            include_forks: true,
            orgs: vec!["acme".to_string(), " ".to_string()],
            exclude_repos: vec!["acme/skip".to_string()],
        },
        workspaces: vec!["~/repos".to_string()],
        finder: FinderConfigDto {
            scan_roots: vec!["~/Code".to_string()],
            max_depth: 5,
            exclude_dirs: vec!["node_modules".to_string(), "target".to_string()],
            show_ambient: false,
        },
        monitor: MonitorConfigDto {
            fullscan_interval_secs: 90,
        },
    })
    .unwrap();
    let loaded = read_app_config().unwrap();

    assert_eq!(saved, loaded);
    assert_eq!(loaded.structure, "{provider}/{org}/{repo}");
    assert_eq!(loaded.sync_mode, "pull");
    assert_eq!(loaded.clone.depth, 1);
    assert_eq!(loaded.filters.orgs, vec!["acme"]);
    assert!(!loaded.finder.show_ambient);
    assert_eq!(loaded.monitor.fullscan_interval_secs, 90);
}

#[test]
fn workspace_save_and_delete_only_remove_metadata() {
    let temp = TestDir::new("workspace-crud");
    let _env = ConfigEnvGuard::new(temp.path());
    ensure_config().unwrap();
    let root = temp.path().join("workspace");
    let repo_file = root.join("acme/widgets/keep.txt");
    std::fs::create_dir_all(repo_file.parent().unwrap()).unwrap();
    std::fs::write(&repo_file, "keep").unwrap();

    let detail = save_workspace(workspace_input(&root)).unwrap();

    assert_eq!(detail.name, "workspace");
    assert!(detail.default);
    assert!(root.join(".git-same/config.toml").exists());
    assert_eq!(
        read_workspace(detail.id.clone()).unwrap().orgs,
        vec!["acme"]
    );

    let workspaces = delete_workspace(detail.id).unwrap();

    assert!(workspaces.is_empty());
    assert!(!root.join(".git-same").exists());
    assert!(repo_file.exists());
    let config = Config::load().unwrap();
    assert!(config.default_workspace.is_none());
    assert!(config.workspaces.is_empty());
}

#[test]
fn set_default_workspace_can_set_and_clear_default() {
    let temp = TestDir::new("workspace-default");
    let _env = ConfigEnvGuard::new(temp.path());
    ensure_config().unwrap();
    let root = temp.path().join("workspace");
    let detail = save_workspace(WorkspaceInput {
        default: false,
        ..workspace_input(&root)
    })
    .unwrap();

    let listed = set_default_workspace(Some(detail.id.clone())).unwrap();
    assert_eq!(listed.len(), 1);
    assert!(listed[0].default);

    let listed = set_default_workspace(None).unwrap();
    assert_eq!(listed.len(), 1);
    assert!(!listed[0].default);
}

#[test]
fn save_workspace_can_clear_existing_default() {
    let temp = TestDir::new("workspace-clear-default");
    let _env = ConfigEnvGuard::new(temp.path());
    ensure_config().unwrap();
    let root = temp.path().join("workspace");
    let detail = save_workspace(workspace_input(&root)).unwrap();

    let updated = save_workspace(WorkspaceInput {
        id: Some(detail.id),
        default: false,
        ..workspace_input(&root)
    })
    .unwrap();

    assert!(!updated.default);
    assert!(Config::load().unwrap().default_workspace.is_none());
}

#[tokio::test]
async fn read_workspace_structure_uses_discovery_cache() {
    let temp = TestDir::new("workspace-structure");
    let _env = ConfigEnvGuard::new(temp.path());
    ensure_config().unwrap();
    let root = temp.path().join("workspace");
    let detail = save_workspace(WorkspaceInput {
        structure: Some("{provider}/{org}/{repo}".to_string()),
        ..workspace_input(&root)
    })
    .unwrap();
    let local_repo = root.join("github/acme/widgets");
    std::fs::create_dir_all(&local_repo).unwrap();

    let repo = git_same_core::types::Repo {
        id: 42,
        name: "widgets".to_string(),
        full_name: "acme/widgets".to_string(),
        ssh_url: "git@github.com:acme/widgets.git".to_string(),
        clone_url: "https://github.com/acme/widgets.git".to_string(),
        default_branch: "main".to_string(),
        private: false,
        archived: false,
        fork: false,
        pushed_at: None,
        description: None,
    };
    let cache = DiscoveryCache::new(
        "manuel".to_string(),
        HashMap::from([("github".to_string(), vec![OwnedRepo::new("acme", repo)])]),
    );
    CacheManager::for_workspace(&root)
        .unwrap()
        .save(&cache)
        .unwrap();

    let structure = read_workspace_structure_inner(detail.id).await.unwrap();

    assert_eq!(structure.source, "cache");
    assert_eq!(structure.host, "github.com");
    assert_eq!(structure.repos.len(), 1);
    assert_eq!(structure.repos[0].full_name, "acme/widgets");
    assert_eq!(structure.repos[0].url, "https://github.com/acme/widgets");
    assert_eq!(
        std::fs::canonicalize(&structure.repos[0].local_path).unwrap(),
        std::fs::canonicalize(&local_repo).unwrap()
    );
    assert!(structure.repos[0].local_exists);
}

#[test]
fn requirement_check_dto_maps_core_result() {
    let dto = requirement_check_dto(CheckResult {
        name: "Git".to_string(),
        passed: true,
        message: "git version 2.0".to_string(),
        suggestion: None,
        critical: true,
    });

    assert_eq!(dto.name, "Git");
    assert!(dto.passed);
    assert_eq!(dto.message, "git version 2.0");
    assert!(dto.critical);
}

#[test]
fn parse_pluginkit_output_marks_enabled_extension() {
    let stdout = "+    com.zaai.git-same.badges(3.1.0)    \
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
    let stdout = "-    com.zaai.git-same.badges(3.1.0)    \
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
