use super::*;

#[test]
fn test_resolved_auth_method_display() {
    assert_eq!(format!("{}", ResolvedAuthMethod::GhCli), "GitHub CLI");
}

#[test]
fn test_extract_host() {
    assert_eq!(
        extract_host("https://api.github.com"),
        Some("api.github.com".to_string())
    );
    assert_eq!(
        extract_host("https://github.company.com/api/v3"),
        Some("github.company.com".to_string())
    );
    assert_eq!(
        extract_host("http://localhost:8080/api"),
        Some("localhost:8080".to_string())
    );
}

#[test]
fn test_extract_host_no_scheme() {
    assert_eq!(
        extract_host("api.github.com/v3"),
        Some("api.github.com".to_string())
    );
}

#[test]
fn test_extract_host_empty() {
    assert_eq!(extract_host(""), None);
}

#[test]
fn test_extract_host_scheme_only() {
    assert_eq!(extract_host("https://"), None);
}

#[test]
fn test_extract_host_with_port() {
    assert_eq!(
        extract_host("https://github.example.com:8443/api/v3"),
        Some("github.example.com:8443".to_string())
    );
}
