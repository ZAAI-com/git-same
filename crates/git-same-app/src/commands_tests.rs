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

fn agent_in(state: MonitorAgentState) -> MonitorLaunchAgentStatusDto {
    MonitorLaunchAgentStatusDto {
        state,
        message: format!("{state:?}"),
        ..MonitorAgentStatus::unsupported()
    }
}

#[test]
fn monitor_requirement_treats_a_long_first_scan_as_healthy() {
    assert!(monitor_is_healthy(&agent_in(MonitorAgentState::Starting)));
    assert!(monitor_is_healthy(&agent_in(MonitorAgentState::Running)));
    assert_eq!(
        monitor_requirement_suggestion(Some(&agent_in(MonitorAgentState::Starting))),
        None
    );
}

#[test]
fn monitor_requirement_does_not_call_an_intentional_stop_broken() {
    let stopped = agent_in(MonitorAgentState::Disabled);
    assert!(!monitor_is_healthy(&stopped));
    assert_eq!(
        monitor_requirement_suggestion(Some(&stopped)),
        Some("Start monitoring to see Finder badges".to_string())
    );
}

#[test]
fn monitor_requirement_prefers_the_concrete_error_detail() {
    let failed = agent_in(MonitorAgentState::Failed).failed("launchctl bootstrap failed");
    assert_eq!(
        monitor_requirement_message(Some(&failed)),
        "launchctl bootstrap failed"
    );
}

fn empty_scan_snapshot() -> StatusSnapshot {
    let mut status = FinderStatus::new(1, chrono::Utc::now().to_rfc3339());
    status.workspaces = vec![git_same_core::types::FinderWorkspaceInfo {
        name: "work".to_string(),
        root: std::path::PathBuf::from("/tmp/work"),
        orgs: Vec::new(),
    }];
    StatusSnapshot {
        status_path: String::new(),
        updated_at: None,
        stale: false,
        status: Some(status),
    }
}

#[test]
fn no_permission_warning_while_the_first_scan_is_running() {
    let snapshot = empty_scan_snapshot();
    assert!(!full_disk_access_needed(
        Some(&agent_in(MonitorAgentState::Starting)),
        Some(&snapshot)
    ));
    assert!(!full_disk_access_needed(None, Some(&snapshot)));
    assert!(full_disk_access_needed(
        Some(&agent_in(MonitorAgentState::Running)),
        Some(&snapshot)
    ));
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
fn read_status_snapshot_freshness_does_not_depend_on_the_monitor_process() {
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

    // The recorded PID is dead, but the data was written just now: process
    // health is the monitor status's business, not the snapshot's.
    assert!(!snapshot.stale);
    assert!(snapshot.updated_at.is_some());
    let status = snapshot
        .status
        .expect("the last-known status must surface from disk");
    assert!(status.repos.is_empty());
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

    let saved = save_app_config_inner(AppConfigInput {
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
        monitor: MonitorConfigInput {
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

/// Spec 7.2: a settings form loaded before `gisa monitor --stop` must not
/// re-enable monitoring when it is saved afterwards. The same write path
/// used to reset `[ui] custom_folder_icon`.
#[test]
fn stale_settings_form_after_cli_stop_keeps_monitoring_stopped() {
    let temp = TestDir::new("stale-form");
    let _env = ConfigEnvGuard::new(temp.path());
    let form = ensure_config().unwrap();
    assert!(form.monitor.autostart);
    let path = std::path::PathBuf::from(&form.config_path);

    // The CLI stops monitoring and the user opts out of folder icons.
    git_same_core::config::edit::set_monitor_autostart(&path, false).unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains("custom_folder_icon = true"));
    std::fs::write(
        &path,
        text.replace("custom_folder_icon = true", "custom_folder_icon = false"),
    )
    .unwrap();

    let saved = save_app_config_inner(AppConfigInput {
        structure: form.structure,
        concurrency: 5,
        sync_mode: form.sync_mode,
        default_workspace: form.default_workspace,
        refresh_interval: form.refresh_interval,
        clone: form.clone,
        filters: form.filters,
        workspaces: form.workspaces,
        finder: form.finder,
        monitor: MonitorConfigInput {
            fullscan_interval_secs: form.monitor.fullscan_interval_secs,
        },
    })
    .unwrap();

    assert_eq!(saved.concurrency, 5);
    assert!(!saved.monitor.autostart, "the CLI stop must survive");
    let config = Config::load_from(&path).unwrap();
    assert!(!config.ui.custom_folder_icon, "[ui] must survive");
}

#[test]
fn saving_settings_never_replaces_a_malformed_config() {
    let temp = TestDir::new("malformed-save");
    let _env = ConfigEnvGuard::new(temp.path());
    let form = ensure_config().unwrap();
    let broken = "concurrency = = 1\n";
    std::fs::write(&form.config_path, broken).unwrap();

    let result = save_app_config_inner(AppConfigInput {
        structure: form.structure,
        concurrency: 5,
        sync_mode: form.sync_mode,
        default_workspace: None,
        refresh_interval: form.refresh_interval,
        clone: form.clone,
        filters: form.filters,
        workspaces: form.workspaces,
        finder: form.finder,
        monitor: MonitorConfigInput {
            fullscan_interval_secs: 30,
        },
    });

    assert!(result.is_err());
    assert_eq!(std::fs::read_to_string(&form.config_path).unwrap(), broken);
}
