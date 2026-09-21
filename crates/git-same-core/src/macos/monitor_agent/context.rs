//! Who we are acting for, and every path the managed monitor uses.
//!
//! [`UserContext::resolve`] is the only place that reads the environment or
//! the passwd database. It refuses to describe anything but the real user's
//! default environment, which is what keeps tests (fake `HOME`, redirected
//! config directory) away from the developer's live launchd domain.

use crate::errors::MonitorAgentError;
use crate::ipc::{IpcConfig, APP_GROUP_ID};
use std::path::{Path, PathBuf};

/// Suppresses automatic monitor management (tests, development).
/// Never reinterprets explicit user commands.
pub const DISABLE_AUTOSTART_ENV: &str = "GIT_SAME_DISABLE_MONITOR_AUTOSTART";

/// Returns `true` when automatic monitor management is suppressed.
pub fn autostart_suppressed() -> bool {
    std::env::var_os(DISABLE_AUTOSTART_ENV).is_some_and(|value| value == "1")
}

/// The user whose monitor is being managed.
#[derive(Debug, Clone)]
pub struct UserContext {
    pub uid: u32,
    pub home: PathBuf,
}

impl UserContext {
    /// Describes the real current user, or explains why managing the service
    /// would be unsafe. `config_override` is `true` when `--config` was given.
    #[cfg(target_os = "macos")]
    pub fn resolve(config_override: bool) -> Result<Self, MonitorAgentError> {
        // SAFETY: getuid cannot fail and has no preconditions.
        let uid = unsafe { libc::getuid() };
        let home = passwd_home(uid).ok_or_else(|| {
            MonitorAgentError::Override(format!("no home directory is recorded for uid {uid}"))
        })?;
        check_overrides(
            uid,
            &home,
            std::env::var_os("HOME").map(PathBuf::from).as_deref(),
            std::env::var_os("GIT_SAME_CONFIG_DIR").is_some(),
            config_override,
        )?;
        Ok(Self { uid, home })
    }

    /// Managed services exist only on macOS.
    #[cfg(not(target_os = "macos"))]
    pub fn resolve(_config_override: bool) -> Result<Self, MonitorAgentError> {
        Err(MonitorAgentError::Unsupported)
    }

    /// Every path derived from this user.
    pub fn paths(&self) -> MonitorAgentPaths {
        MonitorAgentPaths::for_home(&self.home)
    }

    /// `gui/<uid>`: where the agent runs.
    pub fn gui_domain(&self) -> String {
        format!("gui/{}", self.uid)
    }

    /// `user/<uid>`: reachable without a GUI session, for disabled-state queries.
    pub fn user_domain(&self) -> String {
        format!("user/{}", self.uid)
    }
}

/// The tripwires, as a pure function so every one of them is testable.
pub fn check_overrides(
    uid: u32,
    passwd_home: &Path,
    env_home: Option<&Path>,
    config_dir_env_set: bool,
    config_flag_given: bool,
) -> Result<(), MonitorAgentError> {
    if uid == 0 {
        return Err(MonitorAgentError::Override(
            "the monitor runs as a user LaunchAgent and cannot be managed as root".to_string(),
        ));
    }
    if config_flag_given {
        return Err(MonitorAgentError::Override(
            "--config is set, but the background service always uses the default configuration"
                .to_string(),
        ));
    }
    if config_dir_env_set {
        return Err(MonitorAgentError::Override(
            "GIT_SAME_CONFIG_DIR redirects the configuration away from the default".to_string(),
        ));
    }
    if let Some(env_home) = env_home {
        if !same_directory(env_home, passwd_home) {
            return Err(MonitorAgentError::Override(format!(
                "HOME ({}) is not this user's home directory ({})",
                env_home.display(),
                passwd_home.display()
            )));
        }
    }
    Ok(())
}

fn same_directory(a: &Path, b: &Path) -> bool {
    let canonical = |p: &Path| std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
    canonical(a) == canonical(b)
}

#[cfg(target_os = "macos")]
fn passwd_home(uid: u32) -> Option<PathBuf> {
    use std::os::unix::ffi::OsStringExt;
    let mut passwd = std::mem::MaybeUninit::<libc::passwd>::zeroed();
    let mut buffer = vec![0 as libc::c_char; 16 * 1024];
    let mut result: *mut libc::passwd = std::ptr::null_mut();
    // SAFETY: all pointers reference live, correctly sized buffers.
    let status = unsafe {
        libc::getpwuid_r(
            uid,
            passwd.as_mut_ptr(),
            buffer.as_mut_ptr(),
            buffer.len(),
            &mut result,
        )
    };
    if status != 0 || result.is_null() {
        return None;
    }
    // SAFETY: getpwuid_r succeeded, so pw_dir points into `buffer`.
    let dir = unsafe { std::ffi::CStr::from_ptr((*result).pw_dir) };
    let bytes = dir.to_bytes().to_vec();
    (!bytes.is_empty()).then(|| PathBuf::from(std::ffi::OsString::from_vec(bytes)))
}

/// File layout of the managed monitor for one user.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonitorAgentPaths {
    /// `~/Library/Application Support/com.zaai.git-same/monitor`
    pub managed_root: PathBuf,
    /// The helper executable launchd runs.
    pub helper: PathBuf,
    pub install_record: PathBuf,
    pub transaction_record: PathBuf,
    /// Serializes installation, preference changes, and service lifecycle.
    pub control_lock: PathBuf,
    pub launch_agent: PathBuf,
    /// Plist of the pre-rename agent, removed during migration.
    pub legacy_launch_agent: PathBuf,
    pub log_dir: PathBuf,
    pub stdout_log: PathBuf,
    pub stderr_log: PathBuf,
    /// The default user configuration. Managed actions never use another.
    pub config: PathBuf,
    /// App-group directory shared with the Finder extension.
    pub ipc: IpcConfig,
}

impl MonitorAgentPaths {
    pub fn for_home(home: &Path) -> Self {
        let library = home.join("Library");
        let managed_root = library
            .join("Application Support")
            .join("com.zaai.git-same")
            .join("monitor");
        let launch_agents = library.join("LaunchAgents");
        let log_dir = library.join("Logs").join("git-same");
        Self {
            helper: managed_root.join("git-same"),
            install_record: managed_root.join("install.json"),
            transaction_record: managed_root.join("transaction.json"),
            control_lock: managed_root.join("control.lock"),
            launch_agent: launch_agents.join(format!("{}.plist", super::LABEL)),
            legacy_launch_agent: launch_agents.join(format!("{}.plist", super::LEGACY_LABEL)),
            stdout_log: log_dir.join("monitor.log"),
            stderr_log: log_dir.join("monitor.err.log"),
            log_dir,
            config: home.join(".config").join("git-same").join("config.toml"),
            ipc: IpcConfig {
                dir: library.join("Group Containers").join(APP_GROUP_ID),
            },
            managed_root,
        }
    }
}

#[cfg(test)]
#[path = "context_tests.rs"]
mod tests;
