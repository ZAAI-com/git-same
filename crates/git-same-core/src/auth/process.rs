//! Subprocess helpers shared by auth probes.
//!
//! The `gh` CLI and the SSH probe both shell out to external binaries that
//! can stall on network or credential issues. This module provides a single
//! polling-based timeout helper so neither call can block the async runtime
//! or the TUI event loop indefinitely.

use std::io;
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

/// Run `program` with `args` and a hard wall-clock timeout.
///
/// On timeout, the child process is killed and reaped, and an
/// [`io::ErrorKind::TimedOut`] error is returned.
pub(crate) fn run_with_timeout(
    program: &str,
    args: &[&str],
    timeout: Duration,
) -> io::Result<Output> {
    let mut child = Command::new(program)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let deadline = Instant::now() + timeout;
    loop {
        match child.try_wait()? {
            Some(_) => return child.wait_with_output(),
            None => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(io::Error::new(
                        io::ErrorKind::TimedOut,
                        format!(
                            "'{} {}' timed out after {}s",
                            program,
                            args.join(" "),
                            timeout.as_secs()
                        ),
                    ));
                }
                std::thread::sleep(Duration::from_millis(50));
            }
        }
    }
}

#[cfg(test)]
#[path = "process_tests.rs"]
mod tests;
