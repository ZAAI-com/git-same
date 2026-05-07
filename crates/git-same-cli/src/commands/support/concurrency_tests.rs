use super::*;
use git_same_core::output::{Output, Verbosity};

fn quiet_output() -> Output {
    Output::new(Verbosity::Quiet, false)
}

#[test]
fn test_concurrency_within_limit() {
    let output = quiet_output();
    assert_eq!(warn_if_concurrency_capped(4, &output), 4);
}

#[test]
fn test_concurrency_at_limit() {
    let output = quiet_output();
    assert_eq!(
        warn_if_concurrency_capped(MAX_CONCURRENCY, &output),
        MAX_CONCURRENCY
    );
}

#[test]
fn test_concurrency_above_limit() {
    let output = quiet_output();
    assert_eq!(
        warn_if_concurrency_capped(MAX_CONCURRENCY + 10, &output),
        MAX_CONCURRENCY
    );
}
