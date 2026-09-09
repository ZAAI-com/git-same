use super::*;

#[test]
fn classify_maps_success_to_granted() {
    assert_eq!(classify(Ok(())), FullDiskAccess::Granted);
}

#[test]
fn classify_maps_permission_denied_to_denied() {
    // TCC answers EPERM, which std maps to PermissionDenied.
    let denied = io::Error::from_raw_os_error(1);
    assert_eq!(denied.kind(), io::ErrorKind::PermissionDenied);
    assert_eq!(classify(Err(denied)), FullDiskAccess::Denied);
}

#[test]
fn classify_maps_other_errors_to_unknown() {
    let missing = io::Error::new(io::ErrorKind::NotFound, "no TCC.db");
    assert_eq!(classify(Err(missing)), FullDiskAccess::Unknown);
    let other = io::Error::other("disk on fire");
    assert_eq!(classify(Err(other)), FullDiskAccess::Unknown);
}

#[test]
fn is_granted_only_answers_for_definite_states() {
    assert_eq!(FullDiskAccess::Granted.is_granted(), Some(true));
    assert_eq!(FullDiskAccess::Denied.is_granted(), Some(false));
    assert_eq!(FullDiskAccess::Unknown.is_granted(), None);
    assert_eq!(FullDiskAccess::NotApplicable.is_granted(), None);
}

#[test]
fn as_str_labels_are_stable() {
    assert_eq!(FullDiskAccess::Granted.as_str(), "granted");
    assert_eq!(FullDiskAccess::Denied.as_str(), "denied");
    assert_eq!(FullDiskAccess::Unknown.as_str(), "unknown");
    assert_eq!(FullDiskAccess::NotApplicable.as_str(), "not_applicable");
}

#[cfg(target_os = "macos")]
#[test]
fn probe_never_reports_not_applicable_on_macos() {
    // The actual grant depends on the test runner's TCC identity, so only the
    // shape of the answer is asserted: a real macOS probe is never N/A.
    assert_ne!(probe(), FullDiskAccess::NotApplicable);
}

#[cfg(not(target_os = "macos"))]
#[test]
fn probe_reports_not_applicable_off_macos() {
    assert_eq!(probe(), FullDiskAccess::NotApplicable);
    assert_eq!(probe().is_granted(), None);
}
