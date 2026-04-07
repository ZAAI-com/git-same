//! SSH key detection for git operations.
//!
//! Note: SSH keys authenticate git clone/fetch/pull operations,
//! NOT GitHub API calls. This module detects if SSH keys are configured
//! so we can provide better error messages and suggest SSH clone URLs.

use std::path::PathBuf;
use std::process::Command;

/// Outcome of probing SSH connectivity to GitHub.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SshProbeResult {
    /// SSH authenticated successfully.
    Authenticated,
    /// The `ssh` binary was not found on PATH.
    SshNotFound,
    /// SSH key not accepted or no valid key offered.
    PermissionDenied,
    /// Host key verification failed (unknown host, BatchMode prevented prompt).
    HostKeyVerificationFailed,
    /// Network timeout connecting to github.com.
    ConnectionTimeout,
    /// DNS resolution failed.
    DnsFailure,
    /// Unrecognized failure. Carries the stderr output.
    Unknown(String),
}

/// Parse the stderr output of `ssh -T git@github.com` into a diagnostic result.
fn parse_ssh_probe_output(stderr: &str) -> SshProbeResult {
    if stderr.contains("successfully authenticated") {
        return SshProbeResult::Authenticated;
    }
    if stderr.contains("Permission denied") {
        return SshProbeResult::PermissionDenied;
    }
    if stderr.contains("Host key verification failed") {
        return SshProbeResult::HostKeyVerificationFailed;
    }
    if stderr.contains("Could not resolve hostname") {
        return SshProbeResult::DnsFailure;
    }
    if stderr.contains("Connection timed out") || stderr.contains("connect to host") {
        return SshProbeResult::ConnectionTimeout;
    }
    SshProbeResult::Unknown(stderr.to_string())
}

/// Probe SSH connectivity to GitHub and return a diagnostic result.
///
/// Uses BatchMode to avoid interactive prompts. ConnectTimeout=5 prevents
/// hanging on network issues.
pub fn probe_github_ssh() -> SshProbeResult {
    let output = Command::new("ssh")
        .args([
            "-T",
            "-o",
            "BatchMode=yes",
            "-o",
            "ConnectTimeout=5",
            "git@github.com",
        ])
        .output();

    match output {
        Ok(o) => parse_ssh_probe_output(&String::from_utf8_lossy(&o.stderr)),
        Err(_) => SshProbeResult::SshNotFound,
    }
}

/// Check if SSH is likely configured for GitHub.
///
/// Convenience wrapper around [`probe_github_ssh`].
pub fn has_github_ssh_access() -> bool {
    matches!(probe_github_ssh(), SshProbeResult::Authenticated)
}

/// Detect if SSH keys exist in the standard locations.
pub fn has_ssh_keys() -> bool {
    let home = match std::env::var("HOME") {
        Ok(h) => h,
        Err(_) => return false,
    };

    let ssh_dir = PathBuf::from(home).join(".ssh");

    // Check for common SSH key types
    let key_files = [
        "id_rsa",
        "id_ed25519",
        "id_ecdsa",
        "id_dsa",
        "github_rsa",
        "github_ed25519",
    ];

    for key_file in &key_files {
        let key_path = ssh_dir.join(key_file);
        if key_path.exists() {
            return true;
        }
    }

    false
}

/// Get SSH key files that exist.
pub fn get_ssh_key_files() -> Vec<PathBuf> {
    let home = match std::env::var("HOME") {
        Ok(h) => h,
        Err(_) => return vec![],
    };

    let ssh_dir = PathBuf::from(home).join(".ssh");

    let key_files = [
        "id_rsa",
        "id_ed25519",
        "id_ecdsa",
        "id_dsa",
        "github_rsa",
        "github_ed25519",
    ];

    key_files
        .iter()
        .map(|f| ssh_dir.join(f))
        .filter(|p| p.exists())
        .collect()
}

#[cfg(test)]
#[path = "ssh_tests.rs"]
mod tests;
