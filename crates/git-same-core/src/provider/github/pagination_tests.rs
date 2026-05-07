use super::*;

#[test]
fn test_parse_link_header_with_next() {
    let header = r#"<https://api.github.com/user/repos?page=2>; rel="next", <https://api.github.com/user/repos?page=5>; rel="last""#;
    let next = parse_link_header(header);
    assert_eq!(
        next,
        Some("https://api.github.com/user/repos?page=2".to_string())
    );
}

#[test]
fn test_parse_link_header_without_next() {
    let header = r#"<https://api.github.com/user/repos?page=1>; rel="first", <https://api.github.com/user/repos?page=5>; rel="last""#;
    let next = parse_link_header(header);
    assert_eq!(next, None);
}

#[test]
fn test_parse_link_header_only_last() {
    let header = r#"<https://api.github.com/user/repos?page=1>; rel="prev", <https://api.github.com/user/repos?page=5>; rel="last""#;
    let next = parse_link_header(header);
    assert_eq!(next, None);
}

#[test]
fn test_parse_link_header_empty() {
    let next = parse_link_header("");
    assert_eq!(next, None);
}

#[test]
fn test_parse_link_header_malformed() {
    let header = "malformed header without proper format";
    let next = parse_link_header(header);
    assert_eq!(next, None);
}

#[test]
fn test_parse_link_header_complex() {
    let header = r#"<https://api.github.com/organizations/12345/repos?page=2&per_page=100>; rel="next", <https://api.github.com/organizations/12345/repos?page=10&per_page=100>; rel="last""#;
    let next = parse_link_header(header);
    assert_eq!(
        next,
        Some("https://api.github.com/organizations/12345/repos?page=2&per_page=100".to_string())
    );
}

#[test]
fn test_format_reset_time_future() {
    let future = (chrono::Utc::now() + chrono::Duration::minutes(5)).timestamp();
    let result = format_reset_time(&future.to_string());
    assert!(result.contains("UTC"));
    assert!(result.contains("resets in"));
}

#[test]
fn test_format_reset_time_invalid() {
    assert_eq!(format_reset_time("unknown"), "unknown");
}

#[test]
fn test_format_reset_time_empty() {
    assert_eq!(format_reset_time(""), "");
}
