use super::*;
use crate::macos::monitor_agent::fake::{identity, FakeSystem};
use crate::macos::monitor_agent::status::MonitorAgentState;
use crate::types::FinderStatus;
use std::path::PathBuf;

struct Env {
    dir: tempfile::TempDir,
    system: Arc<FakeSystem>,
    user: UserContext,
    paths: MonitorAgentPaths,
    caller: HelperSource,
}

fn write_executable(path: &Path, bytes: &[u8]) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, bytes).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
}

fn cli_source(path: &Path) -> HelperSource {
    HelperSource {
        owner_kind: OwnerKind::Cli,
        owner_path: path.to_path_buf(),
        source_binary: path.to_path_buf(),
        copy_from: path.to_path_buf(),
    }
}

fn env() -> Env {
    let dir = tempfile::tempdir().unwrap();
    let user = UserContext {
        uid: 501,
        home: dir.path().join("home dir"),
    };
    let binary = dir.path().join("dist").join("git-same");
    write_executable(&binary, b"helper v1");
    Env {
        system: Arc::new(FakeSystem::new()),
        paths: user.paths(),
        caller: cli_source(&binary),
        user,
        dir,
    }
}

impl Env {
    fn controller(&self) -> Controller {
        self.controller_for(Some(self.caller.clone()))
    }

    fn controller_for(&self, caller: Option<HelperSource>) -> Controller {
        Controller::new(self.system.clone(), self.user.clone(), caller)
    }

    fn pid(&self) -> Option<u32> {
        self.system.with(|s| s.pids.get(LABEL).copied())
    }

    fn write_config(&self, content: &str) {
        std::fs::create_dir_all(self.paths.config.parent().unwrap()).unwrap();
        std::fs::write(&self.paths.config, content).unwrap();
    }

    /// The running monitor finished its first scan.
    fn complete_scan(&self) {
        let pid = self.system.with(|s| s.active.as_ref().unwrap().pid);
        StatusFileWriter::new(self.paths.ipc.status_file_path())
            .write(&FinderStatus::new(pid, "2026-09-20T10:00:00Z".to_string()))
            .unwrap();
    }

    fn stamps(&self) -> Vec<Option<std::time::SystemTime>> {
        [
            &self.paths.helper,
            &self.paths.launch_agent,
            &self.paths.install_record,
        ]
        .iter()
        .map(|p| std::fs::metadata(p).ok().and_then(|m| m.modified().ok()))
        .collect()
    }

    /// The program the installed agent runs, per the live install record:
    /// the managed copy for a CLI owner, the bundle executable for an app or
    /// cask owner, which is never copied anywhere.
    fn installed_program(&self) -> Option<PathBuf> {
        let record = InstallRecord::load(&self.paths.install_record).ok()??;
        Some(super::source::program_for(
            record.owner_kind,
            &record.owner_path,
            &self.paths.helper,
        ))
    }

    /// The agent is installed and names the program the record implies.
    fn program_installed(&self) -> bool {
        let Some(program) = self.installed_program() else {
            return false;
        };
        std::fs::read_to_string(&self.paths.launch_agent)
            .is_ok_and(|plist| plist.contains(program.to_str().unwrap()))
    }

    /// `(staged installer, final app path, retained service tool)`, matching
    /// what the cask passes: the installer is the staged bundle's CLI helper,
    /// while the program that gets installed is the staged bundle's main
    /// executable, run in place under the app's own TCC identity.
    fn cask_bundle(&self) -> (PathBuf, PathBuf, PathBuf) {
        let bundle = self.dir.path().join("Caskroom/git-same/3.1.2/Git-Same.app");
        let staged = bundle.join("Contents/Helpers/git-same");
        if !staged.exists() {
            write_executable(&staged, b"cask cli v1");
        }
        let staged_app = bundle.join("Contents/MacOS/git-same-app");
        if !staged_app.exists() {
            write_executable(&staged_app, b"cask helper v1");
        }
        let app = self.dir.path().join("Applications").join("Git-Same.app");
        let tool = self
            .dir
            .path()
            .join("Caskroom/git-same/3.1.2/git-same-service-tool");
        (staged, app, tool)
    }
}

// ---------------------------------------------------------------- ensure

#[test]
fn fresh_ensure_installs_and_starts_without_enabling() {
    let env = env();
    let status = env.controller().ensure_running().unwrap();

    assert_eq!(status.state, MonitorAgentState::Starting);
    assert!(status.installed && status.loaded && status.running);
    assert_eq!(status.mode, Some(MonitorMode::Managed));
    assert_eq!(status.pid, env.pid());
    assert_eq!(
        status.helper_version.as_deref(),
        Some(env!("CARGO_PKG_VERSION"))
    );
    assert_eq!(status.owner_kind, Some(OwnerKind::Cli));
    assert!(std::fs::read_to_string(&env.paths.launch_agent)
        .unwrap()
        .contains("--managed"));
    assert!(env.paths.log_dir.is_dir());
    assert!(!env
        .system
        .mutating_calls()
        .iter()
        .any(|call| call.contains("launchctl enable")));
}

#[test]
fn healthy_monitor_is_left_completely_alone() {
    let env = env();
    env.controller().ensure_running().unwrap();
    env.complete_scan();
    let (pid, stamps) = (env.pid(), env.stamps());
    env.system.with(|s| s.calls.clear());

    let status = env.controller().ensure_running().unwrap();

    assert_eq!(status.state, MonitorAgentState::Running);
    assert_eq!(status.last_scan.as_deref(), Some("2026-09-20T10:00:00Z"));
    assert_eq!(env.pid(), pid);
    assert_eq!(env.stamps(), stamps);
    assert!(env.system.mutating_calls().is_empty());
}

#[test]
fn long_initial_scan_stays_starting_and_is_never_restarted() {
    let env = env();
    env.controller().ensure_running().unwrap();
    // A status file left behind by the previous process must not count.
    StatusFileWriter::new(env.paths.ipc.status_file_path())
        .write(&FinderStatus::new(77, "2020-01-01T00:00:00Z".to_string()))
        .unwrap();
    let pid = env.pid();
    env.system.with(|s| s.calls.clear());

    for _ in 0..5 {
        let status = env.controller().ensure_running().unwrap();
        assert_eq!(status.state, MonitorAgentState::Starting);
        assert_eq!(status.last_scan, None);
    }
    assert_eq!(env.pid(), pid);
    assert!(env.system.mutating_calls().is_empty());
}

#[test]
fn missing_helper_is_repaired_from_the_recorded_source() {
    let env = env();
    env.controller().ensure_running().unwrap();
    env.system.with(|s| {
        s.loaded.clear();
        s.pids.clear();
        s.active = None;
    });
    std::fs::remove_file(&env.paths.helper).unwrap();

    let status = env.controller_for(None).ensure_running().unwrap();

    assert!(env.paths.helper.exists());
    assert_eq!(status.state, MonitorAgentState::Starting);
}

#[test]
fn missing_plist_is_rewritten_and_the_service_bootstrapped() {
    let env = env();
    env.controller().ensure_running().unwrap();
    env.system.with(|s| {
        s.loaded.clear();
        s.pids.clear();
        s.active = None;
    });
    std::fs::remove_file(&env.paths.launch_agent).unwrap();

    let status = env.controller().ensure_running().unwrap();

    assert!(env.paths.launch_agent.exists());
    assert!(status.running);
}

#[test]
fn plist_from_the_old_app_bundle_layout_is_repaired() {
    let env = env();
    env.controller().ensure_running().unwrap();
    std::fs::write(
        &env.paths.launch_agent,
        "<plist><string>/Applications/Git-Same.app/Contents/Helpers/git-same</string></plist>",
    )
    .unwrap();

    env.controller().ensure_running().unwrap();

    let plist = std::fs::read_to_string(&env.paths.launch_agent).unwrap();
    assert!(plist.contains(&env.paths.helper.display().to_string()));
    assert!(env.system.with(|s| s.active.is_some()));
}

#[test]
fn unloaded_service_is_bootstrapped() {
    let env = env();
    env.controller().ensure_running().unwrap();
    env.system.with(|s| {
        s.loaded.clear();
        s.pids.clear();
        s.active = None;
        s.calls.clear();
    });

    env.controller().ensure_running().unwrap();

    let calls = env.system.mutating_calls();
    assert_eq!(calls.len(), 1, "{calls:?}");
    assert!(calls[0].starts_with("launchctl bootstrap gui/501 "));
}

#[test]
fn loaded_service_without_a_process_is_kickstarted_without_killing() {
    let env = env();
    env.controller().ensure_running().unwrap();
    env.system.with(|s| {
        s.pids.clear();
        s.active = None;
        s.calls.clear();
    });

    env.controller().ensure_running().unwrap();

    assert_eq!(
        env.system.mutating_calls(),
        vec![format!("launchctl kickstart gui/501/{LABEL}")]
    );
}

#[test]
fn concurrent_ensure_does_not_compete_with_an_operation_in_flight() {
    let env = env();
    let _held = ControlLock::acquire(&env.paths.control_lock, Wait::No, env.system.as_ref());

    let status = env.controller().ensure_running().unwrap();

    assert_eq!(status.state, MonitorAgentState::NotInstalled);
    assert!(!env.paths.helper.exists());
    assert!(env.system.mutating_calls().is_empty());
}

#[test]
fn parallel_ensures_start_exactly_one_monitor() {
    let env = env();
    let handles: Vec<_> = (0..4)
        .map(|_| {
            let controller = env.controller();
            std::thread::spawn(move || controller.ensure_running().map(|s| s.pid))
        })
        .collect();
    for handle in handles {
        handle.join().unwrap().unwrap();
    }
    let bootstraps = env
        .system
        .mutating_calls()
        .iter()
        .filter(|call| call.starts_with("launchctl bootstrap"))
        .count();
    assert_eq!(bootstraps, 1);
}

#[test]
fn headless_session_prepares_the_agent_for_the_next_login() {
    let env = env();
    env.system.with(|s| s.gui = false);

    let status = env.controller().ensure_running().unwrap();

    assert_eq!(status.state, MonitorAgentState::Deferred);
    assert!(env.paths.helper.exists() && env.paths.launch_agent.exists());
    assert!(env.system.mutating_calls().is_empty());
}

#[test]
fn dev_builds_are_never_installed_automatically() {
    let env = env();
    let target = env.dir.path().join("worktree").join("target");
    let binary = target.join("debug").join("git-same");
    write_executable(&binary, b"debug build");
    std::fs::write(target.join("CACHEDIR.TAG"), b"").unwrap();

    let status = env
        .controller_for(Some(cli_source(&binary)))
        .ensure_running()
        .unwrap();

    assert_eq!(status.state, MonitorAgentState::Failed);
    assert!(status.detail.unwrap().contains("development build"));
    assert!(!env.paths.helper.exists());
}

#[test]
fn a_different_cli_never_replaces_a_healthy_helper() {
    let env = env();
    env.controller().ensure_running().unwrap();
    let other = env.dir.path().join("other").join("git-same");
    write_executable(&other, b"another build");
    let pid = env.pid();

    env.controller_for(Some(cli_source(&other)))
        .ensure_running()
        .unwrap();

    assert_eq!(std::fs::read(&env.paths.helper).unwrap(), b"helper v1");
    assert_eq!(env.pid(), pid);
}

#[test]
fn changed_owner_source_updates_the_helper_with_one_restart() {
    let env = env();
    env.controller().ensure_running().unwrap();
    let old_pid = env.pid();
    write_executable(&env.caller.copy_from, b"helper v2 with more bytes");

    env.controller().ensure_running().unwrap();

    assert_eq!(
        std::fs::read(&env.paths.helper).unwrap(),
        b"helper v2 with more bytes"
    );
    assert_ne!(env.pid(), old_pid);
    assert!(env.system.with(|s| s.active.is_some()));
    let record = InstallRecord::load(&env.paths.install_record)
        .unwrap()
        .unwrap();
    assert_eq!(
        record.binary_sha256,
        sha256_file(&env.paths.helper).unwrap()
    );
}

// ------------------------------------------------------- stop / disable

#[test]
fn stop_persists_disables_and_stops() {
    let env = env();
    env.write_config("[monitor]\nautostart = true\n");
    env.controller().ensure_running().unwrap();

    let status = env.controller().stop().unwrap();

    assert_eq!(status.state, MonitorAgentState::Disabled);
    assert!(!status.autostart && status.launchd_disabled && !status.running);
    assert!(!read_monitor_autostart(&env.paths.config).unwrap());
    let calls = env.system.mutating_calls();
    let disable = calls.iter().position(|c| c.contains("disable")).unwrap();
    let bootout = calls.iter().position(|c| c.contains("bootout")).unwrap();
    assert!(disable < bootout, "disable before stopping: {calls:?}");
    assert!(env.paths.helper.exists(), "stop is not uninstall");
}

#[test]
fn stop_never_installs_a_missing_helper() {
    let env = env();
    env.controller().stop().unwrap();
    assert!(!env.paths.helper.exists());
    assert!(!env.paths.launch_agent.exists());
}

#[test]
fn explicit_stop_survives_automatic_recovery() {
    let env = env();
    env.controller().ensure_running().unwrap();
    env.controller().stop().unwrap();
    env.system.with(|s| s.calls.clear());

    let status = env.controller().ensure_running().unwrap();

    assert_eq!(status.state, MonitorAgentState::Disabled);
    assert!(env.system.mutating_calls().is_empty());
}

#[test]
fn native_disabled_state_survives_automatic_recovery() {
    let env = env();
    env.system.with(|s| {
        s.disabled.insert(LABEL.to_string());
    });

    let status = env.controller().ensure_running().unwrap();

    assert_eq!(status.state, MonitorAgentState::Disabled);
    assert!(status.autostart && status.launchd_disabled);
    assert!(env.system.mutating_calls().is_empty());
    assert!(!env.paths.helper.exists());
}

#[test]
fn start_reenables_after_a_stop() {
    let env = env();
    env.controller().stop().unwrap();

    let status = env.controller().start().unwrap();

    assert!(status.running && status.autostart && !status.launchd_disabled);
    assert!(read_monitor_autostart(&env.paths.config).unwrap());
}

#[test]
fn headless_start_enables_for_the_next_login() {
    let env = env();
    env.write_config("[monitor]\nautostart = false\n");
    env.system.with(|s| {
        s.gui = false;
        s.disabled.insert(LABEL.to_string());
    });

    let status = env.controller().start().unwrap();

    assert_eq!(status.state, MonitorAgentState::Deferred);
    assert!(status.installed && status.autostart && !status.launchd_disabled);
    assert!(!status.loaded && !status.running);
    assert!(read_monitor_autostart(&env.paths.config).unwrap());
    assert_eq!(
        env.system.mutating_calls(),
        vec![
            format!("launchctl enable gui/501/{LABEL}"),
            format!("launchctl enable user/501/{LABEL}"),
        ]
    );
}

#[test]
fn start_on_a_healthy_monitor_keeps_its_pid() {
    let env = env();
    env.controller().start().unwrap();
    let pid = env.pid();
    env.controller().start().unwrap();
    assert_eq!(env.pid(), pid);
}

#[test]
fn restart_performs_exactly_one_controlled_restart() {
    let env = env();
    env.controller().start().unwrap();
    let pid = env.pid();
    env.system.with(|s| s.calls.clear());

    let status = env.controller().restart().unwrap();

    assert_ne!(env.pid(), pid);
    assert!(status.running);
    let kicks: Vec<_> = env
        .system
        .mutating_calls()
        .into_iter()
        .filter(|c| c.contains("kickstart"))
        .collect();
    assert_eq!(
        kicks,
        vec![format!("launchctl kickstart -k gui/501/{LABEL}")]
    );
}

#[test]
fn stop_racing_an_installation_waits_for_the_lock_then_wins() {
    let env = env();
    let held = ControlLock::acquire(&env.paths.control_lock, Wait::No, env.system.as_ref());
    let controller = env.controller();
    let stopper = std::thread::spawn(move || controller.stop().map(|s| s.state));
    std::thread::sleep(Duration::from_millis(100));
    drop(held);

    assert_eq!(
        stopper.join().unwrap().unwrap(),
        MonitorAgentState::Disabled
    );
    assert_eq!(
        env.controller().ensure_running().unwrap().state,
        MonitorAgentState::Disabled
    );
}

#[test]
fn malformed_config_is_never_rewritten() {
    let env = env();
    let broken = "[monitor\nautostart = maybe";
    env.write_config(broken);

    let status = env.controller().ensure_running().unwrap();
    assert_eq!(status.state, MonitorAgentState::Failed);
    assert!(env.system.mutating_calls().is_empty());

    // Stop still disables natively, then reports what it could not persist.
    let error = env.controller().stop().unwrap_err();
    assert!(matches!(error, MonitorAgentError::Configuration(_)));
    assert!(env.system.with(|s| s.disabled.contains(LABEL)));

    assert!(env.controller().start().is_err());
    assert_eq!(std::fs::read_to_string(&env.paths.config).unwrap(), broken);
}

// ------------------------------------------------------------ foreground

#[test]
fn foreground_monitor_is_left_running_by_ensure() {
    let env = env();
    env.system.set_foreground_monitor(4321);

    let status = env.controller().ensure_running().unwrap();

    assert_eq!(status.mode, Some(MonitorMode::Foreground));
    assert_eq!(status.pid, Some(4321));
    assert!(env.system.mutating_calls().is_empty());
}

#[test]
fn start_refuses_to_take_over_from_a_foreground_monitor() {
    let env = env();
    env.system.set_foreground_monitor(4321);

    let error = env.controller().start().unwrap_err();

    assert!(matches!(
        error,
        MonitorAgentError::ForegroundActive { pid: 4321 }
    ));
    assert!(env.system.with(|s| s.active.is_some()));
}

#[test]
fn stop_terminates_a_verified_foreground_monitor() {
    let env = env();
    env.system.set_foreground_monitor(4321);

    env.controller().stop().unwrap();

    assert!(env
        .system
        .with(|s| s.calls.contains(&"terminate 4321".to_string())));
    assert!(env.system.with(|s| s.active.is_none()));
}

#[test]
fn a_stale_status_file_pid_never_gets_signalled() {
    let env = env();
    StatusFileWriter::new(env.paths.ipc.status_file_path())
        .write(&FinderStatus::new(std::process::id(), "x".to_string()))
        .unwrap();

    env.controller().stop().unwrap();

    assert!(!env
        .system
        .with(|s| s.calls.iter().any(|c| c.starts_with("terminate"))));
}

// ------------------------------------------------------ failure handling

#[test]
fn bootstrap_failure_restores_the_previous_installation_and_service() {
    let env = env();
    env.controller().ensure_running().unwrap();
    write_executable(&env.caller.copy_from, b"helper v2 that fails to start");
    // The new job is accepted but never becomes a monitor.
    env.system.with(|s| {
        s.fail_verbs.insert(
            "bootstrap".to_string(),
            crate::macos::monitor_agent::system::CommandOutput {
                code: Some(5),
                stdout: String::new(),
                stderr: "Bootstrap failed: 5: Input/output error".to_string(),
            },
        );
    });

    let status = env.controller().ensure_running().unwrap();

    assert_eq!(status.state, MonitorAgentState::Failed);
    let detail = status.detail.unwrap();
    assert!(detail.contains("bootstrap"), "{detail}");
    assert!(detail.contains("rollback also failed"), "{detail}");
    assert_eq!(std::fs::read(&env.paths.helper).unwrap(), b"helper v1");
    assert!(!env.paths.transaction_record.exists());
    let record = InstallRecord::load(&env.paths.install_record)
        .unwrap()
        .unwrap();
    assert_eq!(
        record.binary_sha256,
        sha256_file(&env.paths.helper).unwrap()
    );
}

#[test]
fn start_that_launchd_never_honours_is_reported_with_the_log_path() {
    let env = env();
    env.system.with(|s| s.jobs_never_start = true);

    let error = env.controller().start().unwrap_err().to_string();

    assert!(error.contains("did not start the monitor"), "{error}");
    assert!(error.contains("monitor.err.log"), "{error}");
    assert!(
        !env.paths.helper.exists(),
        "failed first install is rolled back"
    );
    assert!(!env.system.with(|s| s.loaded.contains(LABEL)));
}

#[cfg(unix)]
#[test]
fn install_record_failure_unloads_and_rolls_back_a_first_install() {
    use std::os::unix::ffi::OsStringExt;

    let env = env();
    let mut caller = env.caller.clone();
    caller.owner_path = PathBuf::from(std::ffi::OsString::from_vec(vec![b'/', b'x', 0xff]));

    let error = env.controller_for(Some(caller)).start().unwrap_err();

    assert!(error.to_string().contains("install record"), "{error}");
    assert!(!env.system.with(|s| s.loaded.contains(LABEL)));
    assert!(env.system.with(|s| s.active.is_none()));
    assert!(!env.paths.helper.exists());
    assert!(!env.paths.launch_agent.exists());
    assert!(!env.paths.install_record.exists());
    assert!(!env.paths.transaction_record.exists());
}

#[test]
fn unreadable_install_record_is_reported_not_overwritten() {
    let env = env();
    env.controller().ensure_running().unwrap();
    env.system.with(|s| {
        s.loaded.clear();
        s.pids.clear();
        s.active = None;
    });
    let future = r#"{"schema_version": 9}"#;
    std::fs::write(&env.paths.install_record, future).unwrap();

    let status = env.controller().ensure_running().unwrap();

    assert_eq!(status.state, MonitorAgentState::Failed);
    assert_eq!(
        std::fs::read_to_string(&env.paths.install_record).unwrap(),
        future
    );
}

// ------------------------------------------------------------- uninstall

#[test]
fn uninstall_removes_the_payload_and_keeps_everything_else() {
    let env = env();
    env.controller().start().unwrap();
    env.write_config("# keep me\nconcurrency = 3\n");
    std::fs::write(&env.paths.stderr_log, b"diagnostics").unwrap();

    let status = env.controller().uninstall().unwrap();

    assert_eq!(status.state, MonitorAgentState::Disabled);
    assert!(!env.paths.helper.exists());
    assert!(!env.paths.launch_agent.exists());
    assert!(!env.paths.install_record.exists());
    assert!(env.paths.control_lock.exists());
    assert!(env.paths.stderr_log.exists());
    let config = std::fs::read_to_string(&env.paths.config).unwrap();
    assert!(config.contains("# keep me") && config.contains("autostart = false"));
    assert!(env.system.with(|s| s.loaded.is_empty()));
}

/// The staged bundle's main executable, given its staged CLI helper.
fn staged_app(staged_cli: &Path) -> PathBuf {
    staged_cli
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("MacOS/git-same-app")
}

// ------------------------------------------------------------------ cask

#[test]
fn cask_install_starts_monitoring_without_the_app() {
    let env = env();
    let (staged, app, tool) = env.cask_bundle();

    let status = env
        .controller_for(None)
        .install_for_cask(&staged, &app, &tool)
        .unwrap();

    assert!(status.running);
    assert_eq!(status.owner_kind, Some(OwnerKind::HomebrewCask));
    assert_eq!(status.source.as_deref(), Some(app.to_str().unwrap()));
    assert_eq!(std::fs::read(&tool).unwrap(), b"cask cli v1");
    let record = InstallRecord::load(&env.paths.install_record)
        .unwrap()
        .unwrap();
    assert_eq!(
        record.source_binary,
        app.join("Contents/MacOS/git-same-app"),
        "never the staging path"
    );
    // The monitor runs the bundle executable itself: a copy under the
    // managed root would be a TCC identity the app's grant never reaches.
    assert!(!env.paths.helper.exists());
    assert!(std::fs::read_to_string(&env.paths.launch_agent)
        .unwrap()
        .contains(app.join("Contents/MacOS/git-same-app").to_str().unwrap()));
    assert!(std::fs::read_to_string(&env.paths.launch_agent)
        .unwrap()
        .contains("AssociatedBundleIdentifiers"));
}

#[test]
fn cask_upgrade_after_a_stop_updates_the_helper_but_stays_stopped() {
    let env = env();
    let (staged, app, tool) = env.cask_bundle();
    let controller = env.controller_for(None);
    controller.install_for_cask(&staged, &app, &tool).unwrap();
    controller.stop().unwrap();
    assert!(controller.remove_for_cask(&app).unwrap());
    write_executable(&staged_app(&staged), b"cask helper v2");
    env.system.with(|s| s.calls.clear());

    let status = controller.install_for_cask(&staged, &app, &tool).unwrap();

    assert_eq!(status.state, MonitorAgentState::Disabled);
    assert_eq!(
        std::fs::read(staged_app(&staged)).unwrap(),
        b"cask helper v2"
    );
    assert!(env.system.with(|s| s.active.is_none()));
    let calls = env.system.mutating_calls();
    assert!(!calls.iter().any(|c| c.contains("enable")), "{calls:?}");
    assert!(!calls.iter().any(|c| c.contains("bootstrap")), "{calls:?}");
}

#[test]
fn cask_upgrade_while_enabled_starts_the_new_helper() {
    let env = env();
    let (staged, app, tool) = env.cask_bundle();
    let controller = env.controller_for(None);
    controller.install_for_cask(&staged, &app, &tool).unwrap();
    assert!(controller.remove_for_cask(&app).unwrap());
    assert!(read_monitor_autostart(&env.paths.config).unwrap());
    assert!(env.system.with(|s| s.disabled.is_empty()));
    write_executable(&staged_app(&staged), b"cask helper v2");

    let status = controller.install_for_cask(&staged, &app, &tool).unwrap();

    assert!(status.running);
    assert_eq!(
        std::fs::read(staged_app(&staged)).unwrap(),
        b"cask helper v2"
    );
}

#[test]
fn app_launch_right_after_a_cask_install_changes_nothing() {
    let env = env();
    let (staged, app, tool) = env.cask_bundle();
    env.controller_for(None)
        .install_for_cask(&staged, &app, &tool)
        .unwrap();
    // Homebrew has moved the bundle into place and reopens the app.
    let installed_executable = app.join("Contents/MacOS/git-same-app");
    write_executable(&installed_executable, b"cask helper v1");
    let app_caller = HelperSource {
        owner_kind: OwnerKind::App,
        owner_path: app.clone(),
        source_binary: installed_executable.clone(),
        copy_from: installed_executable,
    };
    let (pid, stamps) = (env.pid(), env.stamps());
    env.system.with(|s| s.calls.clear());

    env.controller_for(Some(app_caller))
        .ensure_running()
        .unwrap();

    assert_eq!(env.pid(), pid);
    assert_eq!(env.stamps(), stamps);
    assert!(env.system.mutating_calls().is_empty());
}

#[test]
fn reinstalling_the_same_cask_keeps_exactly_one_monitor() {
    let env = env();
    let (staged, app, tool) = env.cask_bundle();
    let controller = env.controller_for(None);
    controller.install_for_cask(&staged, &app, &tool).unwrap();
    let pid = env.pid();

    controller.install_for_cask(&staged, &app, &tool).unwrap();

    assert_eq!(env.pid(), pid);
    assert_eq!(env.system.with(|s| s.loaded.len()), 1);
}

#[test]
fn failing_to_retain_the_service_tool_aborts_before_any_live_change() {
    let env = env();
    let (staged, app, tool) = env.cask_bundle();
    env.system
        .with(|s| s.fail_copy_to = Some("git-same-service-tool".to_string()));

    assert!(env
        .controller_for(None)
        .install_for_cask(&staged, &app, &tool)
        .is_err());

    assert!(!env.paths.helper.exists());
    assert!(env.system.mutating_calls().is_empty());
}

#[test]
fn cask_removal_preserves_preference_and_disabled_state() {
    let env = env();
    let (staged, app, tool) = env.cask_bundle();
    let controller = env.controller_for(None);
    controller.install_for_cask(&staged, &app, &tool).unwrap();
    env.system.with(|s| s.calls.clear());

    assert!(controller.remove_for_cask(&app).unwrap());

    assert!(!env.paths.install_record.exists() && !env.paths.launch_agent.exists());
    assert!(env.system.with(|s| s.active.is_none()));
    assert!(read_monitor_autostart(&env.paths.config).unwrap());
    let calls = env.system.mutating_calls();
    assert!(!calls.iter().any(|c| c.contains("disable")), "{calls:?}");
}

#[test]
fn cask_removal_leaves_an_unrelated_installation_untouched() {
    let env = env();
    env.controller().ensure_running().unwrap();
    let (_, app, _) = env.cask_bundle();
    let pid = env.pid();

    assert!(!env.controller_for(None).remove_for_cask(&app).unwrap());

    assert!(env.paths.helper.exists());
    assert_eq!(env.pid(), pid);
}

#[test]
fn cask_removal_for_a_different_app_path_does_nothing() {
    let env = env();
    let (staged, app, tool) = env.cask_bundle();
    let controller = env.controller_for(None);
    controller.install_for_cask(&staged, &app, &tool).unwrap();

    let elsewhere = env.dir.path().join("Other/Git-Same.app");
    assert!(!controller.remove_for_cask(&elsewhere).unwrap());
    assert!(env.program_installed());
}

#[test]
fn cask_removal_with_nothing_installed_succeeds() {
    let env = env();
    let (_, app, _) = env.cask_bundle();
    assert!(!env.controller_for(None).remove_for_cask(&app).unwrap());
}

#[test]
fn cask_removal_refuses_to_guess_when_ownership_is_unreadable() {
    let env = env();
    let (staged, app, tool) = env.cask_bundle();
    let controller = env.controller_for(None);
    controller.install_for_cask(&staged, &app, &tool).unwrap();
    std::fs::write(&env.paths.install_record, "{broken").unwrap();

    let error = controller.remove_for_cask(&app).unwrap_err();

    assert!(matches!(error, MonitorAgentError::OwnershipMismatch(_)));
    assert!(env.paths.launch_agent.exists());
}

#[test]
fn cask_install_with_a_malformed_config_installs_but_does_not_start() {
    let env = env();
    env.write_config("[monitor\n");
    let (staged, app, tool) = env.cask_bundle();

    let result = env
        .controller_for(None)
        .install_for_cask(&staged, &app, &tool);

    assert!(result.is_ok(), "brew install must not fail: {result:?}");
    assert!(env.program_installed());
    assert!(env.system.with(|s| s.active.is_none()));
}

#[test]
fn headless_cask_install_ends_deferred() {
    let env = env();
    env.system.with(|s| s.gui = false);
    let (staged, app, tool) = env.cask_bundle();

    let status = env
        .controller_for(None)
        .install_for_cask(&staged, &app, &tool)
        .unwrap();

    assert_eq!(status.state, MonitorAgentState::Deferred);
}

// ---------------------------------------------------------------- legacy

#[test]
fn legacy_daemon_is_stopped_before_its_plist_is_removed() {
    let env = env();
    std::fs::create_dir_all(env.paths.legacy_launch_agent.parent().unwrap()).unwrap();
    std::fs::write(&env.paths.legacy_launch_agent, "<plist/>").unwrap();
    env.system.with(|s| {
        s.loaded.insert(LEGACY_LABEL.to_string());
        s.pids.insert(LEGACY_LABEL.to_string(), 900);
    });

    env.controller().ensure_running().unwrap();

    assert!(!env.paths.legacy_launch_agent.exists());
    assert!(!env.system.with(|s| s.loaded.contains(LEGACY_LABEL)));
    assert!(env.system.with(|s| s.loaded.contains(LABEL)));
}

#[test]
fn legacy_unload_failure_is_surfaced_and_keeps_the_plist() {
    let env = env();
    std::fs::create_dir_all(env.paths.legacy_launch_agent.parent().unwrap()).unwrap();
    std::fs::write(&env.paths.legacy_launch_agent, "<plist/>").unwrap();
    env.system.with(|s| {
        s.loaded.insert(LEGACY_LABEL.to_string());
        s.fail_verbs.insert(
            "bootout".to_string(),
            crate::macos::monitor_agent::system::CommandOutput {
                code: Some(5),
                stdout: String::new(),
                stderr: "Input/output error".to_string(),
            },
        );
    });

    let status = env.controller().ensure_running().unwrap();

    assert_eq!(status.state, MonitorAgentState::Failed);
    assert!(env.paths.legacy_launch_agent.exists());
}

#[test]
fn missing_legacy_service_is_not_an_error() {
    let env = env();
    std::fs::create_dir_all(env.paths.legacy_launch_agent.parent().unwrap()).unwrap();
    std::fs::write(&env.paths.legacy_launch_agent, "<plist/>").unwrap();

    let status = env.controller().ensure_running().unwrap();

    assert!(status.running);
    assert!(!env.paths.legacy_launch_agent.exists());
}

// --------------------------------------------------------------- inspect

#[test]
fn inspect_modifies_nothing() {
    let env = env();
    let status = env.controller().inspect().unwrap();

    assert_eq!(status.state, MonitorAgentState::NotInstalled);
    assert!(!env.paths.managed_root.exists());
    assert!(!env.paths.config.exists());
    assert!(env.system.mutating_calls().is_empty());
    let _ = identity(1, MonitorMode::Managed);
}

/// The cask's uninstall stanza invokes the retained tool by absolute path.
/// Deleting it before the replacement copy succeeded left `brew uninstall`
/// impossible without `--force`.
#[test]
fn a_failed_retain_keeps_the_previously_retained_service_tool() {
    let env = env();
    let (staged, app, tool) = env.cask_bundle();
    env.controller_for(None)
        .install_for_cask(&staged, &app, &tool)
        .unwrap();
    let retained = std::fs::read(&tool).unwrap();

    env.system
        .with(|s| s.fail_copy_to = Some("git-same-service-tool".to_string()));
    assert!(env
        .controller_for(None)
        .install_for_cask(&staged, &app, &tool)
        .is_err());

    assert_eq!(
        std::fs::read(&tool).unwrap(),
        retained,
        "the working uninstall tool must survive a failed upgrade"
    );
    let leftovers: Vec<_> = std::fs::read_dir(tool.parent().unwrap())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|name| name.ends_with(crate::fsutil::TEMP_SUFFIX))
        .collect();
    assert!(leftovers.is_empty(), "temp copy swept: {leftovers:?}");
}

/// Start refuses while a hand-started monitor runs. It must refuse without
/// having already flipped the persistent preference a Stop had set.
#[test]
fn a_refused_start_does_not_reverse_a_persistent_stop() {
    let env = env();
    env.write_config("[monitor]\nautostart = false\n");
    env.system.set_foreground_monitor(4242);

    let err = env.controller().start().unwrap_err();

    assert!(matches!(err, MonitorAgentError::ForegroundActive { .. }));
    assert!(
        !read_monitor_autostart(&env.paths.config).unwrap(),
        "the preference must still say stopped"
    );
    assert!(env.system.mutating_calls().is_empty());
}

/// An SSH session while the console user is logged in: the GUI domain is
/// unreachable, so `bootout` is skipped. Stop used to spin for the exit
/// timeout and then report failure with the monitor still running, and
/// Uninstall went on to delete the helper from under the live process.
#[test]
fn stop_signals_a_managed_monitor_when_the_gui_domain_is_unreachable() {
    let env = env();
    env.write_config("[monitor]\nautostart = true\n");
    env.controller().ensure_running().unwrap();
    let pid = env.system.with(|s| s.active.as_ref().unwrap().pid);
    env.system.with(|s| {
        s.gui = false;
        s.calls.clear();
    });
    env.system.set_managed_monitor(pid);

    let status = env.controller().stop().unwrap();

    assert!(env
        .system
        .with(|s| s.calls.iter().any(|c| c == &format!("terminate {pid}"))));
    assert!(env.system.with(|s| s.active.is_none()));
    assert!(!status.running);
    assert!(!read_monitor_autostart(&env.paths.config).unwrap());
}

#[test]
fn disable_failure_is_reported_after_the_monitor_is_stopped() {
    let env = env();
    env.write_config("[monitor]\nautostart = true\n");
    env.controller().ensure_running().unwrap();
    env.system.with(|s| {
        s.fail_verbs.insert(
            "disable".to_string(),
            crate::macos::monitor_agent::system::CommandOutput {
                code: Some(5),
                stdout: String::new(),
                stderr: "permission denied".to_string(),
            },
        );
    });

    let error = env.controller().stop().unwrap_err();

    assert!(error.to_string().contains("disable"));
    assert!(env.system.with(|s| s.active.is_none()));
}

#[test]
fn reset_stop_ignores_only_a_malformed_preference_after_shutdown() {
    let env = env();
    env.write_config("[monitor\n");
    env.system.set_foreground_monitor(4242);

    env.controller().stop_before_config_removal().unwrap();

    assert_eq!(
        std::fs::read_to_string(&env.paths.config).unwrap(),
        "[monitor\n"
    );
    assert!(env.system.with(|s| s.active.is_none()));
}

#[test]
fn uninstall_keeps_payload_when_runtime_exit_cannot_be_confirmed() {
    let env = env();
    env.controller().ensure_running().unwrap();
    env.system.with(|s| {
        s.gui = false;
        s.active = None;
        s.runtime_held_unknown = true;
    });

    assert!(env.controller().uninstall().is_err());
    assert!(env.paths.helper.exists());
    assert!(env.paths.launch_agent.exists());
}

#[test]
fn headless_cask_removal_signals_the_managed_monitor() {
    let env = env();
    let (staged, app, tool) = env.cask_bundle();
    env.controller_for(None)
        .install_for_cask(&staged, &app, &tool)
        .unwrap();
    let pid = env.system.with(|s| s.active.as_ref().unwrap().pid);
    env.system.with(|s| s.gui = false);

    assert!(env.controller_for(None).remove_for_cask(&app).unwrap());
    assert!(env.system.with(|s| s
        .calls
        .iter()
        .any(|call| call == &format!("terminate {pid}"))));
    assert!(!env.paths.helper.exists());
}

#[test]
fn corrupted_executable_helper_is_repaired_from_its_recorded_source() {
    let env = env();
    env.controller().ensure_running().unwrap();
    env.system.with(|s| {
        s.loaded.clear();
        s.pids.clear();
        s.active = None;
    });
    write_executable(&env.paths.helper, b"corrupted but executable");

    env.controller_for(None).ensure_running().unwrap();

    assert_eq!(std::fs::read(&env.paths.helper).unwrap(), b"helper v1");
}
