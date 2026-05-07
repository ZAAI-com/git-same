use super::*;

#[test]
fn returns_output_for_fast_command() {
    let output =
        run_with_timeout("true", &[], Duration::from_secs(2)).expect("fast command should succeed");
    assert!(output.status.success());
}

#[test]
fn returns_error_for_missing_binary() {
    let err = run_with_timeout(
        "definitely-not-a-real-binary-xyz",
        &[],
        Duration::from_secs(1),
    )
    .expect_err("missing binary should fail");
    assert_eq!(err.kind(), io::ErrorKind::NotFound);
}

#[test]
fn times_out_and_kills_slow_command() {
    let start = Instant::now();
    let err = run_with_timeout("sleep", &["5"], Duration::from_millis(200))
        .expect_err("slow command should time out");
    assert_eq!(err.kind(), io::ErrorKind::TimedOut);
    assert!(
        start.elapsed() < Duration::from_secs(2),
        "timeout did not kill the child quickly enough"
    );
}
