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
#[ignore] // Requires network access
fn test_has_github_ssh_access() {
    let _ = has_github_ssh_access();
}

#[test]
#[ignore] // Requires network access
fn test_probe_github_ssh_returns_valid_variant() {
    let result = probe_github_ssh();
    match result {
        SshProbeResult::Authenticated
        | SshProbeResult::SshNotFound
        | SshProbeResult::PermissionDenied
        | SshProbeResult::HostKeyVerificationFailed
        | SshProbeResult::ConnectionTimeout
        | SshProbeResult::DnsFailure
        | SshProbeResult::Unknown(_) => {}
    }
}

#[test]
fn test_parse_authenticated() {
    let stderr =
        "Hi user! You've successfully authenticated, but GitHub does not provide shell access.";
    assert_eq!(
        parse_ssh_probe_output(stderr),
        SshProbeResult::Authenticated
    );
}

#[test]
fn test_parse_permission_denied() {
    let stderr = "git@github.com: Permission denied (publickey).";
    assert_eq!(
        parse_ssh_probe_output(stderr),
        SshProbeResult::PermissionDenied
    );
}

#[test]
fn test_parse_host_key_verification_failed() {
    let stderr = "Host key verification failed.";
    assert_eq!(
        parse_ssh_probe_output(stderr),
        SshProbeResult::HostKeyVerificationFailed
    );
}

#[test]
fn test_parse_dns_failure() {
    let stderr = "ssh: Could not resolve hostname github.com: nodename nor servname provided";
    assert_eq!(parse_ssh_probe_output(stderr), SshProbeResult::DnsFailure);
}

#[test]
fn test_parse_connection_timeout() {
    let stderr = "ssh: connect to host github.com port 22: Connection timed out";
    assert_eq!(
        parse_ssh_probe_output(stderr),
        SshProbeResult::ConnectionTimeout
    );
}

#[test]
fn test_parse_connect_to_host_variant() {
    let stderr = "ssh: connect to host github.com port 22: Network is unreachable";
    assert_eq!(
        parse_ssh_probe_output(stderr),
        SshProbeResult::ConnectionTimeout
    );
}

#[test]
fn test_parse_unknown_error() {
    let stderr = "some unexpected error message";
    assert_eq!(
        parse_ssh_probe_output(stderr),
        SshProbeResult::Unknown("some unexpected error message".to_string()),
    );
}
