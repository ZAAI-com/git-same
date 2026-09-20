//! The side-effect boundary of the monitor controller.
//!
//! Everything that leaves the process (external commands, copying an
//! executable, sleeping, signalling, probing the running monitor) goes
//! through [`System`], so controller tests run against a scripted fake and
//! can never reach the real launchd domain.

use crate::errors::MonitorAgentError;
use crate::ipc::IpcConfig;
use crate::monitor::runtime_guard::{self, RuntimeIdentity};
use std::path::Path;
use std::time::Duration;

/// Result of an external command that ran to completion.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CommandOutput {
    /// Exit code; `None` when the process was killed by a signal.
    pub code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

impl CommandOutput {
    pub fn success(&self) -> bool {
        self.code == Some(0)
    }
}

/// Operating-system services used by the controller.
pub trait System: Send + Sync {
    /// Runs a command to completion, killing it after `timeout`.
    fn run(
        &self,
        program: &str,
        args: &[&str],
        timeout: Duration,
    ) -> Result<CommandOutput, MonitorAgentError>;

    /// Copies an executable, preserving permissions, extended attributes,
    /// and its embedded code signature.
    fn copy_executable(&self, from: &Path, to: &Path) -> std::io::Result<()>;

    fn sleep(&self, duration: Duration);

    /// The monitor that verifiably holds the runtime lock, if any.
    fn active_monitor(&self, ipc: &IpcConfig) -> Option<RuntimeIdentity> {
        runtime_guard::active_monitor(ipc)
    }

    /// Asks a process to shut down gracefully.
    fn terminate(&self, pid: u32) -> std::io::Result<()>;
}

/// The real operating system.
#[derive(Debug, Default, Clone, Copy)]
pub struct RealSystem;

impl System for RealSystem {
    fn run(
        &self,
        program: &str,
        args: &[&str],
        timeout: Duration,
    ) -> Result<CommandOutput, MonitorAgentError> {
        run_with_timeout(program, args, timeout)
    }

    fn copy_executable(&self, from: &Path, to: &Path) -> std::io::Result<()> {
        copy_preserving_metadata(from, to)
    }

    fn sleep(&self, duration: Duration) {
        std::thread::sleep(duration);
    }

    fn terminate(&self, pid: u32) -> std::io::Result<()> {
        crate::monitor::process::terminate(pid)
    }
}

/// `std::process::Command` has no timeout, and a wedged `launchctl` must not
/// hang the app or the CLI: poll for exit and kill at the deadline.
fn run_with_timeout(
    program: &str,
    args: &[&str],
    timeout: Duration,
) -> Result<CommandOutput, MonitorAgentError> {
    use std::io::Read;
    use std::process::{Command, Stdio};

    let describe = || format!("{program} {}", args.join(" "));
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| MonitorAgentError::io(format!("Failed to start '{}'", describe()), e))?;

    // Drain the pipes on threads so a chatty child cannot block on a full pipe.
    let drain = |pipe: Option<Box<dyn Read + Send>>| {
        std::thread::spawn(move || {
            let mut text = String::new();
            if let Some(mut pipe) = pipe {
                let mut bytes = Vec::new();
                let _ = pipe.read_to_end(&mut bytes);
                text = String::from_utf8_lossy(&bytes).into_owned();
            }
            text
        })
    };
    let stdout = drain(
        child
            .stdout
            .take()
            .map(|p| Box::new(p) as Box<dyn Read + Send>),
    );
    let stderr = drain(
        child
            .stderr
            .take()
            .map(|p| Box::new(p) as Box<dyn Read + Send>),
    );

    let deadline = std::time::Instant::now() + timeout;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if std::time::Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(MonitorAgentError::CommandTimeout {
                    command: describe(),
                    timeout,
                });
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(20)),
            Err(e) => {
                return Err(MonitorAgentError::io(
                    format!("Failed to wait for '{}'", describe()),
                    e,
                ))
            }
        }
    };

    Ok(CommandOutput {
        code: status.code(),
        stdout: stdout.join().unwrap_or_default(),
        stderr: stderr.join().unwrap_or_default(),
    })
}

/// `copyfile(3)` with data plus metadata flags (`COPYFILE_ALL`) keeps mode bits, extended attributes,
/// and ACLs. The code signature is embedded in the Mach-O, so it travels
/// with the bytes.
#[cfg(target_os = "macos")]
fn copy_preserving_metadata(from: &Path, to: &Path) -> std::io::Result<()> {
    use std::os::unix::ffi::OsStrExt;
    let c_path = |p: &Path| {
        std::ffi::CString::new(p.as_os_str().as_bytes())
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))
    };
    let (from, to) = (c_path(from)?, c_path(to)?);
    // SAFETY: both paths are valid NUL-terminated strings; a null state is allowed.
    let status = unsafe {
        libc::copyfile(
            from.as_ptr(),
            to.as_ptr(),
            std::ptr::null_mut(),
            libc::COPYFILE_METADATA | libc::COPYFILE_DATA,
        )
    };
    if status == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

#[cfg(not(target_os = "macos"))]
fn copy_preserving_metadata(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::copy(from, to).map(|_| ())
}

#[cfg(test)]
#[path = "system_tests.rs"]
mod tests;
