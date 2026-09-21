//! Scripted [`System`] for controller tests: a tiny launchd simulation plus
//! recorded calls. Nothing here can reach the real launchd domain.

use super::system::{CommandOutput, System};
use crate::errors::MonitorAgentError;
use crate::ipc::IpcConfig;
use crate::monitor::runtime_guard::{MonitorMode, RuntimeIdentity, RuntimeMonitorState};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

#[derive(Default)]
pub struct FakeState {
    pub gui: bool,
    pub loaded: HashSet<String>,
    pub pids: HashMap<String, u32>,
    pub disabled: HashSet<String>,
    /// Every external command, as one space-joined string.
    pub calls: Vec<String>,
    pub next_pid: u32,
    /// Verified holder of the runtime lock.
    pub active: Option<RuntimeIdentity>,
    pub runtime_held_unknown: bool,
    /// launchctl verbs that fail with the given output.
    pub fail_verbs: HashMap<String, CommandOutput>,
    /// Copies whose destination contains this text fail.
    pub fail_copy_to: Option<String>,
    /// Copies silently produce different bytes.
    pub corrupt_copies: bool,
    /// launchd accepts the job but its process never appears.
    pub jobs_never_start: bool,
    /// Scripted `codesign`. `None` keeps the default: every binary is
    /// unsigned, which is true of test binaries and short-circuits
    /// `verify_signature` before it can check anything.
    pub codesign: Option<FakeCodesign>,
    pub slept: Duration,
}

/// Enough of `codesign` to exercise the helper verification path, which is
/// otherwise dead code under test.
#[derive(Debug, Clone, Default)]
pub struct FakeCodesign {
    pub team: String,
    /// Path substrings whose binaries carry the app-group entitlement.
    pub app_group: Vec<String>,
    /// Path substrings whose `--verify --strict` fails.
    pub verify_fails: Vec<String>,
    /// Path substrings whose entitlement read fails outright.
    pub entitlements_fail: Vec<String>,
}

impl FakeCodesign {
    /// A correctly signed helper carrying the app group.
    pub fn signed(team: &str) -> Self {
        Self {
            team: team.to_string(),
            app_group: vec![String::new()],
            ..Self::default()
        }
    }

    fn matches(patterns: &[String], path: &str) -> bool {
        patterns.iter().any(|p| path.contains(p.as_str()))
    }

    fn respond(&self, args: &[&str]) -> CommandOutput {
        let path = args[args.len() - 1];
        if args[0] == "-dv" {
            return CommandOutput {
                code: Some(0),
                stdout: String::new(),
                stderr: format!("TeamIdentifier={}\n", self.team),
            };
        }
        if args[0] == "--verify" {
            return if Self::matches(&self.verify_fails, path) {
                fail(1, "invalid signature")
            } else {
                ok("")
            };
        }
        // -d --entitlements - --xml
        if Self::matches(&self.entitlements_fail, path) {
            return fail(1, "cannot read entitlements");
        }
        if Self::matches(&self.app_group, path) {
            ok(format!("<string>{}</string>", crate::ipc::APP_GROUP_ID))
        } else {
            ok("")
        }
    }
}

pub struct FakeSystem {
    pub state: Mutex<FakeState>,
}

impl FakeSystem {
    pub fn new() -> Self {
        Self {
            state: Mutex::new(FakeState {
                gui: true,
                next_pid: 1000,
                ..FakeState::default()
            }),
        }
    }

    pub fn with<R>(&self, f: impl FnOnce(&mut FakeState) -> R) -> R {
        f(&mut self.state.lock().unwrap())
    }

    /// Calls that change launchd state (everything except queries).
    pub fn mutating_calls(&self) -> Vec<String> {
        self.with(|s| {
            s.calls
                .iter()
                .filter(|call| {
                    ["bootstrap", "bootout", "kickstart", "enable", "disable"]
                        .iter()
                        .any(|verb| call.starts_with(&format!("launchctl {verb}")))
                })
                .cloned()
                .collect()
        })
    }

    pub fn set_foreground_monitor(&self, pid: u32) {
        self.with(|s| s.active = Some(identity(pid, MonitorMode::Foreground)));
    }

    /// A managed monitor that launchd started before the GUI domain became
    /// unreachable -- the state an SSH session sees while the console user is
    /// still logged in. `spawn` cannot produce it, because it requires `gui`.
    pub fn set_managed_monitor(&self, pid: u32) {
        self.with(|s| {
            s.loaded.insert(super::LABEL.to_string());
            s.pids.insert(super::LABEL.to_string(), pid);
            s.active = Some(identity(pid, MonitorMode::Managed));
        });
    }
}

pub fn identity(pid: u32, mode: MonitorMode) -> RuntimeIdentity {
    RuntimeIdentity {
        pid,
        start_identity: None,
        executable: PathBuf::new(),
        mode,
        started_at: String::new(),
    }
}

fn ok(stdout: impl Into<String>) -> CommandOutput {
    CommandOutput {
        code: Some(0),
        stdout: stdout.into(),
        stderr: String::new(),
    }
}

fn fail(code: i32, stderr: &str) -> CommandOutput {
    CommandOutput {
        code: Some(code),
        stdout: String::new(),
        stderr: stderr.to_string(),
    }
}

fn label_of(target: &str) -> String {
    target.rsplit('/').next().unwrap_or_default().to_string()
}

impl FakeState {
    fn spawn(&mut self, label: &str) {
        if self.jobs_never_start {
            return;
        }
        self.next_pid += 1;
        let pid = self.next_pid;
        self.pids.insert(label.to_string(), pid);
        if label == super::LABEL {
            self.active = Some(identity(pid, MonitorMode::Managed));
        }
    }

    fn launchctl(&mut self, args: &[&str]) -> CommandOutput {
        let verb = args[0];
        if let Some(output) = self.fail_verbs.get(verb) {
            return output.clone();
        }
        match verb {
            "print" => {
                let target = args[1];
                if target.matches('/').count() == 1 {
                    return if self.gui {
                        ok("")
                    } else {
                        fail(113, "Could not find domain")
                    };
                }
                let label = label_of(target);
                if !self.gui || !self.loaded.contains(&label) {
                    return fail(113, "Could not find service");
                }
                match self.pids.get(&label) {
                    Some(pid) => ok(format!(
                        "{target} = {{\n\tstate = running\n\tpid = {pid}\n}}\n"
                    )),
                    None => ok(format!("{target} = {{\n\tstate = not running\n}}\n")),
                }
            }
            "print-disabled" => {
                if args[1].starts_with("gui/") && !self.gui {
                    return fail(113, "Could not find domain");
                }
                let mut out = String::from("disabled services = {\n");
                for label in &self.disabled {
                    out.push_str(&format!("\t\"{label}\" => disabled\n"));
                }
                out.push_str("}\n");
                ok(out)
            }
            "bootstrap" => {
                let label = Path::new(args[2])
                    .file_stem()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_default();
                if !self.gui {
                    return fail(125, "Domain does not support specified action");
                }
                if self.disabled.contains(&label) {
                    return fail(5, "Bootstrap failed: 5: Input/output error");
                }
                if !self.loaded.insert(label.clone()) {
                    return fail(37, "Operation already in progress");
                }
                self.spawn(&label);
                ok("")
            }
            "bootout" => {
                let label = label_of(args[1]);
                if !self.loaded.remove(&label) {
                    return fail(3, "Boot-out failed: 3: No such process");
                }
                let pid = self.pids.remove(&label);
                if self.active.as_ref().map(|a| a.pid) == pid {
                    self.active = None;
                }
                ok("")
            }
            "kickstart" => {
                let restart = args[1] == "-k";
                let label = label_of(args[args.len() - 1]);
                if !self.loaded.contains(&label) {
                    return fail(113, "Could not find service");
                }
                if restart || !self.pids.contains_key(&label) {
                    self.spawn(&label);
                }
                ok("")
            }
            "enable" => {
                if args[1].starts_with("gui/") && !self.gui {
                    return fail(113, "Could not find domain");
                }
                self.disabled.remove(&label_of(args[1]));
                ok("")
            }
            "disable" => {
                if args[1].starts_with("gui/") && !self.gui {
                    return fail(113, "Could not find domain");
                }
                self.disabled.insert(label_of(args[1]));
                ok("")
            }
            other => fail(64, &format!("unknown verb {other}")),
        }
    }
}

impl System for FakeSystem {
    fn run(
        &self,
        program: &str,
        args: &[&str],
        _timeout: Duration,
    ) -> Result<CommandOutput, MonitorAgentError> {
        let name = program.rsplit('/').next().unwrap_or(program);
        let mut state = self.state.lock().unwrap();
        state.calls.push(format!("{name} {}", args.join(" ")));
        Ok(match name {
            "launchctl" => state.launchctl(args),
            "codesign" => match state.codesign.clone() {
                Some(codesign) => codesign.respond(args),
                // Test binaries are unsigned.
                None => fail(1, "code object is not signed at all"),
            },
            _ => ok(""),
        })
    }

    fn copy_executable(&self, from: &Path, to: &Path) -> std::io::Result<()> {
        let (fail_to, corrupt) = self.with(|s| (s.fail_copy_to.clone(), s.corrupt_copies));
        if fail_to.is_some_and(|text| to.to_string_lossy().contains(&text)) {
            return Err(std::io::Error::other("simulated copy failure"));
        }
        std::fs::copy(from, to)?;
        if corrupt {
            std::fs::write(to, b"corrupted")?;
        }
        Ok(())
    }

    fn sleep(&self, duration: Duration) {
        self.with(|s| s.slept += duration);
        // Virtual time, but yield so other test threads can make progress.
        std::thread::sleep(Duration::from_millis(2));
    }

    fn active_monitor(&self, _ipc: &IpcConfig) -> Option<RuntimeIdentity> {
        self.with(|s| s.active.clone())
    }

    fn monitor_state(&self, _ipc: &IpcConfig) -> RuntimeMonitorState {
        self.with(|s| match &s.active {
            Some(identity) => RuntimeMonitorState::Active(identity.clone()),
            None if s.runtime_held_unknown => RuntimeMonitorState::HeldUnknown,
            None => RuntimeMonitorState::Stopped,
        })
    }

    fn terminate(&self, pid: u32) -> std::io::Result<()> {
        self.with(|s| {
            s.calls.push(format!("terminate {pid}"));
            s.pids.retain(|_, running_pid| *running_pid != pid);
            if s.active.as_ref().map(|a| a.pid) == Some(pid) {
                s.active = None;
            }
        });
        Ok(())
    }

    fn terminate_monitor(&self, identity: &RuntimeIdentity) -> std::io::Result<()> {
        self.terminate(identity.pid)
    }
}
