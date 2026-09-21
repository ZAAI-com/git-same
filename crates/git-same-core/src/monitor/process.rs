//! Process liveness, identity, and signalling.
//!
//! A PID alone is never proof of identity: PIDs are reused. `start_identity`
//! captures when the process started, so a recorded PID can be matched
//! against the process that currently holds it.

/// Returns `true` when `pid` can be a real process ID.
///
/// Rejects 0 and anything above `i32::MAX`: `u32::MAX` would reach `kill` as
/// -1, which addresses every process the caller may signal.
pub fn is_valid_pid(pid: u32) -> bool {
    pid != 0 && pid <= i32::MAX as u32
}

/// Returns `true` when a process with `pid` exists.
#[cfg(unix)]
pub fn is_alive(pid: u32) -> bool {
    if !is_valid_pid(pid) {
        return false;
    }
    // SAFETY: signal 0 performs error checking only and sends nothing.
    let result = unsafe { libc::kill(pid as libc::pid_t, 0) };
    // EPERM means the process exists but belongs to someone else.
    result == 0 || std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
}

/// Without POSIX signals liveness cannot be probed, so assume a valid PID is
/// alive rather than report a false negative.
#[cfg(not(unix))]
pub fn is_alive(pid: u32) -> bool {
    is_valid_pid(pid)
}

/// Asks `pid` to shut down gracefully (SIGTERM).
#[cfg(unix)]
pub fn terminate(pid: u32) -> std::io::Result<()> {
    if !is_valid_pid(pid) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "invalid process ID",
        ));
    }
    // SAFETY: plain syscall on a validated, positive PID.
    if unsafe { libc::kill(pid as libc::pid_t, libc::SIGTERM) } == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

/// Signalling is not available on this platform.
#[cfg(not(unix))]
pub fn terminate(_pid: u32) -> std::io::Result<()> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "stopping a monitor is not supported on this platform",
    ))
}

/// Revalidates the recorded process generation immediately before signalling.
pub fn terminate_identity(identity: &super::runtime_guard::RuntimeIdentity) -> std::io::Result<()> {
    if !super::runtime_guard::identity_matches_live_process(identity) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "monitor process identity is no longer active",
        ));
    }
    terminate(identity.pid)
}

/// Returns an opaque token identifying when `pid` started, if obtainable.
#[cfg(target_os = "macos")]
pub fn start_identity(pid: u32) -> Option<String> {
    if !is_valid_pid(pid) {
        return None;
    }
    let mut info = std::mem::MaybeUninit::<libc::proc_bsdinfo>::zeroed();
    let size = std::mem::size_of::<libc::proc_bsdinfo>() as libc::c_int;
    // SAFETY: the buffer is sized for proc_bsdinfo and the kernel writes at
    // most `size` bytes into it.
    let written = unsafe {
        libc::proc_pidinfo(
            pid as libc::c_int,
            libc::PROC_PIDTBSDINFO,
            0,
            info.as_mut_ptr().cast(),
            size,
        )
    };
    if written != size {
        return None;
    }
    // SAFETY: the kernel filled the whole struct.
    let info = unsafe { info.assume_init() };
    Some(format!(
        "{}.{:06}",
        info.pbi_start_tvsec, info.pbi_start_tvusec
    ))
}

/// Returns an opaque token identifying when `pid` started, if obtainable.
#[cfg(target_os = "linux")]
pub fn start_identity(pid: u32) -> Option<String> {
    if !is_valid_pid(pid) {
        return None;
    }
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    // The command name may contain spaces and parentheses; fields resume
    // after the last ')'. `starttime` is field 22, i.e. index 19 after it.
    let rest = &stat[stat.rfind(')')? + 1..];
    rest.split_whitespace().nth(19).map(str::to_string)
}

/// Start identity is not obtainable on this platform.
#[cfg(not(any(target_os = "macos", target_os = "linux")))]
pub fn start_identity(_pid: u32) -> Option<String> {
    None
}

#[cfg(test)]
#[path = "process_tests.rs"]
mod tests;
