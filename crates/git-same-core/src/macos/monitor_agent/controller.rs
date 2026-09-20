//! Lifecycle controller: inspect, ensure, start, stop, restart, uninstall,
//! and the two private cask operations.

use super::context::{MonitorAgentPaths, UserContext};
use super::control_lock::{ControlLock, Wait};
use super::install::{is_executable, Installer, Staged};
use super::launchd::Launchd;
use super::record::{file_stamp, sha256_file, InstallRecord, OwnerKind};
use super::source::{self, HelperSource, Selection, APP_BUNDLE_ID};
use super::status::{derive_state, state_message, Facts, MonitorAgentStatus};
use super::system::System;
use super::{plist, LABEL, LEGACY_LABEL, OBSOLETE_FINDER_EXTENSION_ID};
use crate::config::edit::{read_monitor_autostart, set_monitor_autostart};
use crate::errors::MonitorAgentError;
use crate::ipc::StatusFileWriter;
use crate::monitor::runtime_guard::{MonitorMode, RuntimeIdentity};
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

type Result<T> = std::result::Result<T, MonitorAgentError>;

/// Explicit commands wait this long for a competing operation.
const EXPLICIT_WAIT: Duration = Duration::from_secs(20);
const POLL: Duration = Duration::from_millis(200);
/// How long launchd gets to report a PID after a start request.
const START_CONFIRM: Duration = Duration::from_secs(5);
/// How long a monitor gets to exit and release the runtime lock.
const EXIT_WAIT: Duration = Duration::from_secs(15);

pub struct Controller {
    system: Arc<dyn System>,
    user: UserContext,
    paths: MonitorAgentPaths,
    /// The invoking app or CLI, when it could be resolved.
    caller: Option<HelperSource>,
    version: String,
}

/// Whether an operation was asked for by the user or happens on its own.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Intent {
    Automatic,
    Explicit,
}

impl Controller {
    pub fn new(system: Arc<dyn System>, user: UserContext, caller: Option<HelperSource>) -> Self {
        let paths = user.paths();
        Self {
            system,
            user,
            paths,
            caller,
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    pub fn paths(&self) -> &MonitorAgentPaths {
        &self.paths
    }

    fn launchd(&self) -> Launchd<'_> {
        Launchd::new(self.system.as_ref(), &self.user)
    }

    fn installer(&self) -> Installer<'_> {
        Installer::new(self.system.as_ref(), &self.paths)
    }

    fn lock(&self, wait: Wait) -> Result<ControlLock> {
        ControlLock::acquire(&self.paths.control_lock, wait, self.system.as_ref())
    }

    // ------------------------------------------------------------------
    // inspect
    // ------------------------------------------------------------------

    /// Reads service, helper, preference, and runtime state. Modifies nothing.
    pub fn inspect(&self) -> Result<MonitorAgentStatus> {
        let (autostart, config_error) = match read_monitor_autostart(&self.paths.config) {
            Ok(autostart) => (autostart, None),
            Err(e) => (true, Some(e.to_string())),
        };
        let launchd = self.launchd();
        let record = InstallRecord::load(&self.paths.install_record);
        let active = self.system.active_monitor(&self.paths.ipc);
        let facts = Facts {
            autostart,
            launchd_disabled: launchd.is_disabled(LABEL)?,
            installed: is_executable(&self.paths.helper) && self.paths.launch_agent.exists(),
            gui_session: launchd.gui_session_available()?,
            service: launchd.service(LABEL)?,
            scan_complete: active
                .as_ref()
                .is_some_and(|identity| self.scan_complete(identity)),
            active,
        };
        let state = derive_state(&facts);
        let record_ok = record.as_ref().ok().and_then(|r| r.as_ref());
        let status = MonitorAgentStatus {
            label: LABEL.to_string(),
            plist_path: self.paths.launch_agent.display().to_string(),
            binary_path: self
                .paths
                .helper
                .exists()
                .then(|| self.paths.helper.display().to_string()),
            installed: facts.installed,
            loaded: facts.service.loaded,
            running: facts.active.is_some(),
            state,
            message: state_message(state, &facts),
            pid: facts
                .active
                .as_ref()
                .map(|identity| identity.pid)
                .or(facts.service.pid),
            mode: facts.active.as_ref().map(|identity| identity.mode),
            autostart,
            launchd_disabled: facts.launchd_disabled,
            helper_version: record_ok.map(|r| r.binary_version.clone()),
            source: record_ok.map(|r| r.owner_path.display().to_string()),
            owner_kind: record_ok.map(|r| r.owner_kind),
            last_scan: facts
                .scan_complete
                .then(|| self.last_scan_timestamp())
                .flatten(),
            detail: None,
        };
        // A running monitor is never reported as failed; problems with the
        // configuration or the record surface once it is not running.
        if facts.active.is_none() {
            if let Some(error) = config_error {
                return Ok(status.failed(error));
            }
            if let Err(error) = record {
                return Ok(status.failed(error.to_string()));
            }
        }
        Ok(status)
    }

    /// The active process has written `status.json` itself. A file left by a
    /// previous process carries that process's PID.
    fn scan_complete(&self, identity: &RuntimeIdentity) -> bool {
        StatusFileWriter::new(self.paths.ipc.status_file_path())
            .read()
            .is_ok_and(|status| status.daemon_pid == identity.pid)
    }

    fn last_scan_timestamp(&self) -> Option<String> {
        StatusFileWriter::new(self.paths.ipc.status_file_path())
            .read()
            .ok()
            .map(|status| status.timestamp)
    }

    /// Whether the managed helper may run: `monitor.autostart` is on and
    /// launchd has not disabled the service. The helper checks this before
    /// its first side effect and exits successfully when it is off.
    pub fn monitoring_enabled(&self) -> Result<bool> {
        let autostart = read_monitor_autostart(&self.paths.config)
            .map_err(|e| MonitorAgentError::Configuration(e.to_string()))?;
        Ok(autostart && !self.launchd().is_disabled(LABEL)?)
    }

    // ------------------------------------------------------------------
    // ensure / start / restart
    // ------------------------------------------------------------------

    /// Repairs enabled monitoring. Preserves explicit disabling, never waits
    /// behind another operation, and leaves a healthy monitor untouched.
    pub fn ensure_running(&self) -> Result<MonitorAgentStatus> {
        let _lock = match self.lock(Wait::No) {
            Ok(lock) => lock,
            // Someone else is already installing, starting, or stopping.
            Err(MonitorAgentError::Busy) => return self.inspect(),
            Err(e) => return Err(e),
        };
        self.installer().recover_interrupted()?;

        let autostart = match read_monitor_autostart(&self.paths.config) {
            Ok(autostart) => autostart,
            // A malformed config is reported, never repaired or overridden.
            Err(_) => return self.inspect(),
        };
        if !autostart || self.launchd().is_disabled(LABEL)? {
            return self.inspect();
        }
        match self.bring_up(Intent::Automatic, false) {
            Ok(()) => self.inspect(),
            Err(e) => Ok(self.inspect()?.failed(e.to_string())),
        }
    }

    /// Enables autostart, installs or repairs the helper, and starts it.
    pub fn start(&self) -> Result<MonitorAgentStatus> {
        self.start_inner(false)
    }

    /// Like [`Self::start`], then performs one controlled service restart.
    pub fn restart(&self) -> Result<MonitorAgentStatus> {
        self.start_inner(true)
    }

    fn start_inner(&self, restart: bool) -> Result<MonitorAgentStatus> {
        let _lock = self.lock(Wait::UpTo(EXPLICIT_WAIT))?;
        self.installer().recover_interrupted()?;

        set_monitor_autostart(&self.paths.config, true)
            .map_err(|e| MonitorAgentError::Configuration(e.to_string()))?;
        if let Some(active) = self.system.active_monitor(&self.paths.ipc) {
            if active.mode == MonitorMode::Foreground {
                // Never silently kill a monitor the user started by hand.
                return Err(MonitorAgentError::ForegroundActive { pid: active.pid });
            }
        }
        self.launchd().enable(LABEL)?;
        self.bring_up(Intent::Explicit, restart)?;
        self.inspect()
    }

    /// Steps shared by ensure, start, and restart. Requires the control lock
    /// and an enabled service.
    fn bring_up(&self, intent: Intent, restart: bool) -> Result<()> {
        self.migrate_legacy_agent()?;

        let active = self.system.active_monitor(&self.paths.ipc);
        if active
            .as_ref()
            .is_some_and(|identity| identity.mode == MonitorMode::Foreground)
        {
            return Ok(());
        }

        let record = InstallRecord::load(&self.paths.install_record)?;
        let helper_intact = is_executable(&self.paths.helper);
        let selection = source::select(
            record.as_ref(),
            helper_intact,
            self.caller.as_ref(),
            intent == Intent::Explicit,
            source_changed,
        );

        let launchd = self.launchd();
        match selection {
            Selection::Install(source) => {
                let enabled_start = true;
                self.install(source, enabled_start)?;
                if restart {
                    // A fresh install has just started a new process.
                    return Ok(());
                }
            }
            Selection::Keep => {
                let expected = self.render_plist(record.as_ref().map(|r| r.owner_kind));
                let plist_current = std::fs::read_to_string(&self.paths.launch_agent)
                    .is_ok_and(|current| current == expected);
                if !plist_current {
                    self.rewrite_plist(&expected)?;
                } else if active.is_some() && !restart {
                    // Healthy and current: no writes, no restart.
                    return Ok(());
                }
            }
            Selection::Unavailable(reason) => {
                if !helper_intact {
                    return Err(MonitorAgentError::MissingSource(reason));
                }
            }
        }

        if !launchd.gui_session_available()? {
            // The plist is in place; launchd starts it at the next login.
            return Ok(());
        }
        let service = launchd.service(LABEL)?;
        if !service.loaded {
            launchd.bootstrap(LABEL, &self.paths.launch_agent)?;
        } else if restart {
            launchd.kickstart(LABEL, true)?;
        } else if service.pid.is_none() {
            launchd.kickstart(LABEL, false)?;
        }
        self.confirm_started()
    }

    /// launchd accepted the service and reports a process. Does not wait for
    /// repository scanning: a long first scan is `starting`, not a failure.
    fn confirm_started(&self) -> Result<()> {
        let launchd = self.launchd();
        let mut waited = Duration::ZERO;
        loop {
            let service = launchd.service(LABEL)?;
            if service.pid.is_some() || self.system.active_monitor(&self.paths.ipc).is_some() {
                return Ok(());
            }
            if waited >= START_CONFIRM {
                return Err(MonitorAgentError::Launchd {
                    operation: "start".to_string(),
                    code: None,
                    detail: format!(
                        "launchd did not start the monitor; see {}",
                        self.paths.stderr_log.display()
                    ),
                });
            }
            self.system.sleep(POLL);
            waited += POLL;
        }
    }

    // ------------------------------------------------------------------
    // installation
    // ------------------------------------------------------------------

    fn render_plist(&self, owner_kind: Option<OwnerKind>) -> String {
        let associated = owner_kind
            .is_some_and(OwnerKind::is_app)
            .then_some(APP_BUNDLE_ID);
        plist::render(&self.paths, &self.user.home, associated)
    }

    /// Transactional helper replacement. On failure the previous files and
    /// the previous service state are restored.
    fn install(&self, source: HelperSource, start_if_possible: bool) -> Result<()> {
        let installer = self.installer();
        let staged = installer.stage(source)?;
        std::fs::create_dir_all(&self.paths.log_dir)
            .map_err(|e| MonitorAgentError::io("Failed to create the log directory", e))?;

        let launchd = self.launchd();
        let gui = launchd.gui_session_available()?;
        let was_loaded = gui && launchd.service(LABEL)?.loaded;

        let result = self.replace_and_start(&staged, gui, was_loaded, start_if_possible);
        let Err(original) = result else {
            return installer.commit(&staged, &self.version);
        };

        // Restore files first, then the service state that was live before.
        let mut rollback = installer.rollback().err().map(|e| e.to_string());
        if was_loaded && self.paths.launch_agent.exists() {
            let _ = launchd.bootout(LABEL);
            if let Err(e) = launchd.bootstrap(LABEL, &self.paths.launch_agent) {
                let message = format!("could not restart the previous service: {e}");
                rollback = Some(match rollback {
                    Some(previous) => format!("{previous}; {message}"),
                    None => message,
                });
            }
        }
        Err(MonitorAgentError::Transaction {
            original: original.to_string(),
            rollback,
        })
    }

    fn replace_and_start(
        &self,
        staged: &Staged,
        gui: bool,
        was_loaded: bool,
        start_if_possible: bool,
    ) -> Result<()> {
        let launchd = self.launchd();
        if was_loaded {
            launchd.bootout(LABEL)?;
        }
        self.wait_for_managed_exit()?;
        self.installer()
            .activate(staged, &self.render_plist(Some(staged.source.owner_kind)))?;
        let foreground_active = self.system.active_monitor(&self.paths.ipc).is_some();
        if start_if_possible && gui && !foreground_active {
            launchd.bootstrap(LABEL, &self.paths.launch_agent)?;
            self.confirm_started()?;
        }
        Ok(())
    }

    /// launchd caches the job definition, so a changed plist needs a reload.
    fn rewrite_plist(&self, content: &str) -> Result<()> {
        let launchd = self.launchd();
        if launchd.gui_session_available()? && launchd.service(LABEL)?.loaded {
            launchd.bootout(LABEL)?;
            self.wait_for_managed_exit()?;
        }
        std::fs::create_dir_all(&self.paths.log_dir)
            .map_err(|e| MonitorAgentError::io("Failed to create the log directory", e))?;
        crate::fsutil::atomic_write(&self.paths.launch_agent, content.as_bytes(), Some(0o644))
            .map_err(|e| MonitorAgentError::io("Failed to write the LaunchAgent", e))
    }

    /// After a bootout, wait for the old managed process to release the
    /// runtime lock so two monitors never overlap.
    fn wait_for_managed_exit(&self) -> Result<()> {
        let mut waited = Duration::ZERO;
        while let Some(active) = self.system.active_monitor(&self.paths.ipc) {
            if active.mode == MonitorMode::Foreground {
                return Ok(());
            }
            if waited >= EXIT_WAIT {
                return Err(MonitorAgentError::Launchd {
                    operation: "bootout".to_string(),
                    code: None,
                    detail: format!("monitor (PID {}) did not exit", active.pid),
                });
            }
            self.system.sleep(POLL);
            waited += POLL;
        }
        Ok(())
    }

    /// Stops the pre-rename daemon agent and removes its plist, but only
    /// once launchd confirms it is no longer active.
    fn migrate_legacy_agent(&self) -> Result<()> {
        let launchd = self.launchd();
        let plist_exists = self.paths.legacy_launch_agent.exists();
        if launchd.gui_session_available()? {
            let loaded = launchd.service(LEGACY_LABEL)?.loaded;
            if !loaded && !plist_exists {
                return Ok(());
            }
            if loaded {
                launchd.bootout(LEGACY_LABEL)?;
            }
            if launchd.service(LEGACY_LABEL)?.loaded {
                return Err(MonitorAgentError::Launchd {
                    operation: "bootout".to_string(),
                    code: None,
                    detail: format!("legacy agent {LEGACY_LABEL} is still loaded"),
                });
            }
        }
        if plist_exists {
            std::fs::remove_file(&self.paths.legacy_launch_agent)
                .map_err(|e| MonitorAgentError::io("Failed to remove the legacy LaunchAgent", e))?;
        }
        Ok(())
    }

    // ------------------------------------------------------------------
    // stop / uninstall
    // ------------------------------------------------------------------

    /// Persistently disables and stops monitoring. Survives app launches,
    /// CLI use, upgrades, and restarts until an explicit Start.
    pub fn stop(&self) -> Result<MonitorAgentStatus> {
        let _lock = self.lock(Wait::UpTo(EXPLICIT_WAIT))?;
        self.installer().recover_interrupted()?;
        let persisted = self.stop_locked();
        let status = self.inspect()?;
        match persisted {
            Ok(()) => Ok(status),
            Err(e) => Err(e),
        }
    }

    /// Returns the persistence error, if any, only after the native
    /// disabling and stopping were still attempted.
    fn stop_locked(&self) -> Result<()> {
        // Never overwrites a malformed config: the edit refuses it.
        let persisted = set_monitor_autostart(&self.paths.config, false).map_err(|e| {
            MonitorAgentError::Configuration(format!(
                "monitoring was stopped, but the preference could not be saved: {e}"
            ))
        });

        let launchd = self.launchd();
        // Disable before stopping so KeepAlive cannot bring it back.
        launchd.disable(LABEL)?;
        if launchd.gui_session_available()? {
            launchd.bootout(LABEL)?;
        }
        if let Some(active) = self.system.active_monitor(&self.paths.ipc) {
            if active.mode == MonitorMode::Foreground {
                self.system.terminate(active.pid).map_err(|e| {
                    MonitorAgentError::io("Failed to stop the foreground monitor", e)
                })?;
            }
            self.wait_for_any_exit()?;
        }
        persisted
    }

    fn wait_for_any_exit(&self) -> Result<()> {
        let mut waited = Duration::ZERO;
        while let Some(active) = self.system.active_monitor(&self.paths.ipc) {
            if waited >= EXIT_WAIT {
                return Err(MonitorAgentError::Launchd {
                    operation: "stop".to_string(),
                    code: None,
                    detail: format!("monitor (PID {}) did not exit", active.pid),
                });
            }
            self.system.sleep(POLL);
            waited += POLL;
        }
        Ok(())
    }

    /// Persistent Stop, then removes the helper, its metadata, leftovers, and
    /// the LaunchAgent. Keeps repositories, configuration, and logs.
    pub fn uninstall(&self) -> Result<MonitorAgentStatus> {
        let _lock = self.lock(Wait::UpTo(EXPLICIT_WAIT))?;
        let installer = self.installer();
        installer.recover_interrupted()?;
        let persisted = self.stop_locked();
        installer.remove_installation()?;
        persisted?;
        self.inspect()
    }

    // ------------------------------------------------------------------
    // private cask interface
    // ------------------------------------------------------------------

    /// Installs or updates the helper from a staged cask bundle. Preserves
    /// the startup preference: starts only when monitoring is enabled, and
    /// never calls `launchctl enable`.
    ///
    /// `retained_tool` receives a copy of the running installer so the
    /// cask's uninstall stanza still works after the app is gone. If that
    /// copy fails, nothing live is changed.
    pub fn install_for_cask(
        &self,
        staged_executable: &Path,
        final_app_path: &Path,
        retained_tool: &Path,
    ) -> Result<MonitorAgentStatus> {
        let _ = std::fs::remove_file(retained_tool);
        self.system
            .copy_executable(staged_executable, retained_tool)
            .map_err(|e| {
                MonitorAgentError::io(
                    format!(
                        "Failed to retain the service tool at '{}'",
                        retained_tool.display()
                    ),
                    e,
                )
            })?;

        let _lock = self.lock(Wait::UpTo(EXPLICIT_WAIT))?;
        self.installer().recover_interrupted()?;
        self.migrate_legacy_agent()?;
        self.clear_obsolete_finder_extension();

        // Unknown preference (malformed config) means: install, do not start.
        let enabled = read_monitor_autostart(&self.paths.config).unwrap_or(false)
            && !self.launchd().is_disabled(LABEL)?;

        let source = source::cask_source(staged_executable, final_app_path);
        if self.cask_install_is_current(&source)? {
            if enabled {
                self.bring_up_installed()?;
            }
        } else {
            self.install(source, enabled)?;
        }
        self.inspect()
    }

    /// Same owner, same helper bytes, same plist: nothing to replace.
    fn cask_install_is_current(&self, source: &HelperSource) -> Result<bool> {
        let Some(record) = InstallRecord::load(&self.paths.install_record)? else {
            return Ok(false);
        };
        if record.owner_kind != OwnerKind::HomebrewCask
            || record.owner_path != source.owner_path
            || !is_executable(&self.paths.helper)
        {
            return Ok(false);
        }
        let staged_hash = sha256_file(&source.copy_from)
            .map_err(|e| MonitorAgentError::io("Failed to hash the staged helper", e))?;
        let plist_current = std::fs::read_to_string(&self.paths.launch_agent)
            .is_ok_and(|current| current == self.render_plist(Some(OwnerKind::HomebrewCask)));
        Ok(staged_hash == record.binary_sha256 && plist_current)
    }

    fn bring_up_installed(&self) -> Result<()> {
        let launchd = self.launchd();
        if !launchd.gui_session_available()?
            || self.system.active_monitor(&self.paths.ipc).is_some()
        {
            return Ok(());
        }
        let service = launchd.service(LABEL)?;
        if !service.loaded {
            launchd.bootstrap(LABEL, &self.paths.launch_agent)?;
        } else if service.pid.is_none() {
            launchd.kickstart(LABEL, false)?;
        }
        self.confirm_started()
    }

    /// Best-effort: ignored when the identifier is unknown to pluginkit.
    fn clear_obsolete_finder_extension(&self) {
        let _ = self.system.run(
            "/usr/bin/pluginkit",
            &["-e", "ignore", "-i", OBSOLETE_FINDER_EXTENSION_ID],
            Duration::from_secs(10),
        );
    }

    /// Removes the installation owned by the cask at `final_app_path`.
    ///
    /// Leaves the autostart preference and the launchd-disabled state as
    /// they are, so an upgrade does not change whether monitoring is on.
    /// An installation owned by anyone else is left untouched.
    pub fn remove_for_cask(&self, final_app_path: &Path) -> Result<bool> {
        let _lock = self.lock(Wait::UpTo(EXPLICIT_WAIT))?;
        let installer = self.installer();
        installer.recover_interrupted()?;

        let record = InstallRecord::load(&self.paths.install_record).map_err(|e| {
            MonitorAgentError::OwnershipMismatch(format!(
                "cannot establish who owns the installed monitor: {e}"
            ))
        })?;
        let Some(record) = record else {
            return Ok(false);
        };
        if record.owner_kind != OwnerKind::HomebrewCask || record.owner_path != final_app_path {
            return Ok(false);
        }

        let launchd = self.launchd();
        if launchd.gui_session_available()? {
            launchd.bootout(LABEL)?;
        }
        self.wait_for_managed_exit()?;
        installer.remove_installation()?;
        Ok(true)
    }
}

/// Cheap change check first (size and mtime), hash only when it differs.
fn source_changed(record: &InstallRecord) -> bool {
    let Some(stamp) = file_stamp(&record.source_binary) else {
        return false;
    };
    if record.source_stamp == Some(stamp) {
        return false;
    }
    sha256_file(&record.source_binary).is_ok_and(|hash| hash != record.binary_sha256)
}

#[cfg(test)]
#[path = "controller_tests.rs"]
mod tests;
