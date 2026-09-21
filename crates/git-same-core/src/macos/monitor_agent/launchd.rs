//! launchd adapter for the one Git-Same service.
//!
//! Uses only the domain-explicit verbs (`bootstrap`, `bootout`, `kickstart`,
//! `enable`, `disable`, `print`, `print-disabled`); never the legacy
//! `load` / `unload`. The service lives in `gui/<uid>`. The uid always comes
//! from the resolved user and is never zero.

use super::context::UserContext;
use super::system::{CommandOutput, System};
use crate::errors::MonitorAgentError;
use std::path::Path;
use std::time::Duration;

const LAUNCHCTL: &str = "/bin/launchctl";
const TIMEOUT: Duration = Duration::from_secs(15);

/// What launchd knows about a service.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ServiceInfo {
    /// The job is bootstrapped into the GUI domain.
    pub loaded: bool,
    /// PID when launchd has a live process for it.
    pub pid: Option<u32>,
    /// Executable launchd would run.
    pub program: Option<String>,
}

pub struct Launchd<'a> {
    system: &'a dyn System,
    user: &'a UserContext,
}

impl<'a> Launchd<'a> {
    pub fn new(system: &'a dyn System, user: &'a UserContext) -> Self {
        Self { system, user }
    }

    fn target(&self, label: &str) -> String {
        format!("{}/{label}", self.user.gui_domain())
    }

    fn run(&self, args: &[&str]) -> Result<CommandOutput, MonitorAgentError> {
        self.system.run(LAUNCHCTL, args, TIMEOUT)
    }

    /// A GUI login session exists for this user. Without one the agent can
    /// only be prepared for the next login.
    pub fn gui_session_available(&self) -> Result<bool, MonitorAgentError> {
        let output = self.run(&["print", &self.user.gui_domain()])?;
        if output.success() {
            Ok(true)
        } else if is_not_found(&output) {
            Ok(false)
        } else {
            Err(failure("print GUI domain", &output))
        }
    }

    pub fn service(&self, label: &str) -> Result<ServiceInfo, MonitorAgentError> {
        let output = self.run(&["print", &self.target(label)])?;
        if output.success() {
            return Ok(parse_service(&output.stdout));
        }
        if is_not_found(&output) {
            Ok(ServiceInfo::default())
        } else {
            Err(failure("print service", &output))
        }
    }

    /// The user (or an explicit Stop) disabled the service in launchd.
    /// Falls back to the user domain so the answer survives a missing GUI
    /// session.
    pub fn is_disabled(&self, label: &str) -> Result<bool, MonitorAgentError> {
        for (index, domain) in [self.user.gui_domain(), self.user.user_domain()]
            .into_iter()
            .enumerate()
        {
            let output = self.run(&["print-disabled", &domain])?;
            if output.success() {
                return Ok(parse_disabled(&output.stdout, label));
            }
            if index == 0 && is_not_found(&output) {
                continue;
            }
            return Err(failure("print-disabled", &output));
        }
        unreachable!("the user-domain query either returns or errors")
    }

    /// Loads the job. Succeeds when it is already loaded.
    pub fn bootstrap(&self, label: &str, plist: &Path) -> Result<(), MonitorAgentError> {
        let plist = plist.display().to_string();
        let output = self.run(&["bootstrap", &self.user.gui_domain(), &plist])?;
        if output.success() || self.service(label)?.loaded {
            return Ok(());
        }
        Err(failure("bootstrap", &output))
    }

    /// Unloads the job. `Ok(false)` when it was not loaded; real failures
    /// are surfaced.
    pub fn bootout(&self, label: &str) -> Result<bool, MonitorAgentError> {
        let output = self.run(&["bootout", &self.target(label)])?;
        if output.success() {
            return Ok(true);
        }
        if is_not_found(&output) || !self.service(label)?.loaded {
            return Ok(false);
        }
        Err(failure("bootout", &output))
    }

    /// Starts the job if it is not running. With `restart`, kills and
    /// restarts a running instance (explicit Restart only).
    pub fn kickstart(&self, label: &str, restart: bool) -> Result<(), MonitorAgentError> {
        let target = self.target(label);
        let output = if restart {
            self.run(&["kickstart", "-k", &target])?
        } else {
            self.run(&["kickstart", &target])?
        };
        if output.success() {
            Ok(())
        } else {
            Err(failure("kickstart", &output))
        }
    }

    /// Clears the disabled state. Only explicit Start and Restart call this.
    pub fn enable(&self, label: &str) -> Result<(), MonitorAgentError> {
        let output = self.run(&["enable", &self.target(label)])?;
        if output.success() {
            return Ok(());
        }
        if !is_not_found(&output) {
            return Err(failure("enable", &output));
        }
        let user_target = format!("{}/{label}", self.user.user_domain());
        let fallback = self.run(&["enable", &user_target])?;
        if fallback.success() {
            return Ok(());
        }
        Err(failure("enable", &fallback))
    }

    /// Persists the disabled state. Without a GUI session the GUI domain is
    /// unreachable, so the user domain records it instead.
    pub fn disable(&self, label: &str) -> Result<(), MonitorAgentError> {
        let output = self.run(&["disable", &self.target(label)])?;
        if output.success() {
            return Ok(());
        }
        if !is_not_found(&output) {
            return Err(failure("disable", &output));
        }
        let user_target = format!("{}/{label}", self.user.user_domain());
        let fallback = self.run(&["disable", &user_target])?;
        if fallback.success() {
            return Ok(());
        }
        Err(failure("disable", &fallback))
    }
}

fn failure(operation: &str, output: &CommandOutput) -> MonitorAgentError {
    MonitorAgentError::Launchd {
        operation: operation.to_string(),
        code: output.code,
        detail: format!("{}{}", output.stdout.trim(), output.stderr.trim()),
    }
}

/// launchctl reports an absent service as ESRCH (3) or 113.
fn is_not_found(output: &CommandOutput) -> bool {
    let text = format!("{}{}", output.stdout, output.stderr);
    matches!(output.code, Some(3) | Some(113))
        || text.contains("No such process")
        || text.contains("Could not find service")
}

fn parse_service(stdout: &str) -> ServiceInfo {
    let mut info = ServiceInfo {
        loaded: true,
        ..ServiceInfo::default()
    };
    for line in stdout.lines() {
        // Only top-level keys: nested dictionaries are indented further.
        let Some(line) = line.strip_prefix('\t') else {
            continue;
        };
        if line.starts_with('\t') {
            continue;
        }
        if let Some(value) = line.strip_prefix("pid = ") {
            info.pid = value.trim().parse().ok();
        } else if let Some(value) = line.strip_prefix("program = ") {
            info.program = Some(value.trim().to_string());
        }
    }
    info
}

/// Matches `"label" => disabled` (current) and `"label" => true` (older).
fn parse_disabled(stdout: &str, label: &str) -> bool {
    let needle = format!("\"{label}\"");
    stdout
        .lines()
        .filter(|line| line.contains(&needle))
        .filter_map(|line| line.split("=>").nth(1))
        .any(|state| matches!(state.trim(), "disabled" | "true"))
}

#[cfg(test)]
#[path = "launchd_tests.rs"]
mod tests;
