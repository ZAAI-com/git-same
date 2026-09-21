use super::*;

#[cfg(unix)]
#[test]
fn run_captures_exit_code_and_output() {
    let output = RealSystem
        .run(
            "/bin/sh",
            &["-c", "echo out; echo err 1>&2; exit 3"],
            Duration::from_secs(10),
        )
        .unwrap();
    assert_eq!(output.code, Some(3));
    assert_eq!(output.stdout.trim(), "out");
    assert_eq!(output.stderr.trim(), "err");
    assert!(!output.success());
}

#[cfg(unix)]
#[test]
fn run_kills_a_command_that_exceeds_its_timeout() {
    let started = std::time::Instant::now();
    let error = RealSystem
        .run("/bin/sh", &["-c", "sleep 30"], Duration::from_millis(200))
        .unwrap_err();
    assert!(matches!(error, MonitorAgentError::CommandTimeout { .. }));
    assert!(started.elapsed() < Duration::from_secs(10));
}

#[test]
fn run_reports_a_missing_program() {
    let error = RealSystem
        .run("/nonexistent/program", &[], Duration::from_secs(1))
        .unwrap_err();
    assert!(matches!(error, MonitorAgentError::Io { .. }));
}

#[cfg(unix)]
#[test]
fn copy_preserves_the_executable_bit() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let from = dir.path().join("source");
    let to = dir.path().join("copy");
    std::fs::write(&from, b"#!/bin/sh\n").unwrap();
    std::fs::set_permissions(&from, std::fs::Permissions::from_mode(0o755)).unwrap();

    RealSystem.copy_executable(&from, &to).unwrap();

    assert_eq!(std::fs::read(&to).unwrap(), b"#!/bin/sh\n");
    assert_eq!(
        std::fs::metadata(&to).unwrap().permissions().mode() & 0o777,
        0o755
    );
}
