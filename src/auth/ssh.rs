//! SSH key detection for git operations.
//!
//! Note: SSH keys authenticate git clone/fetch/pull operations,
//! NOT GitHub API calls. This module detects if SSH keys are configured
//! so we can provide better error messages and suggest SSH clone URLs.

use std::path::PathBuf;
use std::process::Command;

/// Check if SSH is likely configured for GitHub.
///
/// Uses BatchMode to avoid interactive prompts. If the host key is not
/// already known, this returns false (user should run `ssh -T git@github.com`
/// manually to verify and accept the host key).
pub fn has_github_ssh_access() -> bool {
    // Try to test SSH connection to GitHub
    // BatchMode=yes prevents interactive prompts (for host key verification, passwords, etc.)
    // ConnectTimeout=5 prevents hanging on network issues
    let output = Command::new("ssh")
        .args([
            "-T",
            "-o", "BatchMode=yes",
            "-o", "ConnectTimeout=5",
            "git@github.com",
        ])
        .output();

    if let Ok(output) = output {
        // GitHub SSH test returns exit code 1 with success message
        // "Hi username! You've successfully authenticated..."
        let stderr = String::from_utf8_lossy(&output.stderr);
        stderr.contains("successfully authenticated")
    } else {
        false
    }
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

/// Check if SSH agent is running.
pub fn has_ssh_agent() -> bool {
    std::env::var("SSH_AUTH_SOCK").is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_has_ssh_keys_detection() {
        // This test just checks that the function runs without panicking
        // The actual result depends on the test environment
        let _ = has_ssh_keys();
    }

    #[test]
    fn test_get_ssh_key_files() {
        // This test just checks that the function runs without panicking
        let keys = get_ssh_key_files();
        // Can't assert specific results as it depends on test environment
        assert!(keys.len() <= 6); // At most 6 key types
    }

    #[test]
    fn test_has_ssh_agent() {
        // This test just checks that the function runs without panicking
        let _ = has_ssh_agent();
    }

    #[test]
    #[ignore] // Ignore by default as it requires network access
    fn test_has_github_ssh_access() {
        // This test requires actual SSH configuration
        let _ = has_github_ssh_access();
    }
}
