//! Probe whether this process holds Full Disk Access (FDA).
//!
//! macOS exposes no API for the `kTCCServiceSystemPolicyAllFiles` grant, so
//! the probe opens a file that every account has and that TCC guards behind
//! FDA: the user's own TCC database. FDA is grant-only (there is no consent
//! dialog), so the open never triggers a prompt and the probe is silent.
//!
//! TCC keys the grant on the calling executable's code identity, so the result
//! describes *this* process. The monitor stamps its own result into
//! `status.json` (the authoritative answer for "can the monitor read protected
//! folders"), and the Tauri host probes its own identity. Running `gisa` from
//! a terminal reports the terminal's grant, not Git-Same's.
//!
//! On non-macOS targets the probe reports [`FullDiskAccess::NotApplicable`].

use std::io;

/// Outcome of a Full Disk Access probe for the current process.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FullDiskAccess {
    /// The process can read TCC-protected locations without prompting.
    Granted,
    /// TCC silently denied the probe (EPERM): no grant for this identity.
    Denied,
    /// The probe could not tell (for example the probe file is missing).
    Unknown,
    /// Not a macOS build; TCC does not apply.
    NotApplicable,
}

impl FullDiskAccess {
    /// `Some(true)` when granted, `Some(false)` when denied, `None` when the
    /// state is unknown or not applicable.
    pub fn is_granted(self) -> Option<bool> {
        match self {
            Self::Granted => Some(true),
            Self::Denied => Some(false),
            Self::Unknown | Self::NotApplicable => None,
        }
    }

    /// Stable lowercase label for serialisation to hosts.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Granted => "granted",
            Self::Denied => "denied",
            Self::Unknown => "unknown",
            Self::NotApplicable => "not_applicable",
        }
    }
}

/// Probe the current process's Full Disk Access state. Never prompts.
pub fn probe() -> FullDiskAccess {
    #[cfg(target_os = "macos")]
    {
        classify(open_probe_file())
    }
    #[cfg(not(target_os = "macos"))]
    {
        FullDiskAccess::NotApplicable
    }
}

/// Open the user TCC database read-only. Success proves FDA; TCC answers with
/// EPERM otherwise. The handle is dropped immediately: nothing is read.
#[cfg(target_os = "macos")]
fn open_probe_file() -> io::Result<()> {
    let home = std::env::var_os("HOME")
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "HOME is not set"))?;
    let path = std::path::PathBuf::from(home)
        .join("Library")
        .join("Application Support")
        .join("com.apple.TCC")
        .join("TCC.db");
    std::fs::File::open(path).map(|_| ())
}

/// Map the probe's open result to a grant state. `PermissionDenied` (EPERM)
/// is TCC's silent deny. Any other failure (missing database, unset HOME)
/// cannot distinguish "no grant" from "nothing to probe", so it is reported as
/// unknown rather than denied.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub(crate) fn classify(result: io::Result<()>) -> FullDiskAccess {
    match result {
        Ok(()) => FullDiskAccess::Granted,
        Err(error) if error.kind() == io::ErrorKind::PermissionDenied => FullDiskAccess::Denied,
        Err(_) => FullDiskAccess::Unknown,
    }
}

#[cfg(test)]
#[path = "full_disk_access_tests.rs"]
mod tests;
