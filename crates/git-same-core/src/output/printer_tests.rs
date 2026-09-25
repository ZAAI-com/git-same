use super::*;

#[test]
fn test_verbosity_from_u8() {
    assert_eq!(Verbosity::from(0), Verbosity::Quiet);
    assert_eq!(Verbosity::from(1), Verbosity::Normal);
    assert_eq!(Verbosity::from(2), Verbosity::Verbose);
    assert_eq!(Verbosity::from(3), Verbosity::Debug);
    assert_eq!(Verbosity::from(100), Verbosity::Debug);
}

#[test]
fn test_verbosity_ordering() {
    assert!(Verbosity::Quiet < Verbosity::Normal);
    assert!(Verbosity::Normal < Verbosity::Verbose);
    assert!(Verbosity::Verbose < Verbosity::Debug);
}

#[test]
fn test_output_creation() {
    let output = Output::new(Verbosity::Normal, false);
    assert!(!output.is_json());

    let json_output = Output::new(Verbosity::Normal, true);
    assert!(json_output.is_json());
}

#[test]
fn test_output_quiet() {
    let output = Output::quiet();
    assert_eq!(output.verbosity, Verbosity::Quiet);
}

#[test]
fn test_format_functions() {
    let count = format_count(42, "repos");
    assert!(count.contains("42"));
    assert!(count.contains("repos"));

    let success = format_success("done");
    assert!(success.contains("done"));

    let error = format_error("failed");
    assert!(error.contains("failed"));

    let warning = format_warning("caution");
    assert!(warning.contains("caution"));
}

#[test]
fn test_summary_shows_by_default_but_not_when_quiet_requested_or_json() {
    // The CLI default level is Quiet, yet summaries must show.
    assert!(Output::new(Verbosity::Quiet, false).shows_summary());
    assert!(!Output::new(Verbosity::Quiet, false)
        .with_quiet_requested(true)
        .shows_summary());
    assert!(!Output::new(Verbosity::Normal, true).shows_summary());
    assert!(!Output::quiet().shows_summary());
    assert!(Output::quiet().is_quiet_requested());
}
