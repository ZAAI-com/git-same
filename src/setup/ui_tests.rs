use super::*;

#[test]
fn center_cell_matches_width() {
    let out = center_cell("Auth", 10);
    assert_eq!(out.chars().count(), 10);
    assert!(out.contains("Auth"));
}

#[test]
fn connector_cell_matches_width() {
    assert_eq!(connector_cell(7, true).chars().count(), 7);
    assert_eq!(connector_cell(7, false).chars().count(), 7);
}
