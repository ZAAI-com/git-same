use super::*;

#[test]
fn test_is_installed_returns_bool() {
    // This test just verifies the function runs without panicking
    // The actual result depends on whether gh is installed
    let _result = is_installed();
}

#[test]
fn test_is_authenticated_returns_bool() {
    let _result = is_authenticated();
}

// Integration tests that require gh to be installed and authenticated
// These are ignored by default
#[test]
#[ignore]
fn test_get_token_when_authenticated() {
    if !is_installed() || !is_authenticated() {
        return;
    }

    let token = get_token().unwrap();
    assert!(!token.is_empty());
    // GitHub tokens start with specific prefixes
    assert!(
        token.starts_with("ghp_")
            || token.starts_with("github_pat_")
            || token.starts_with("gho_")
            || token.starts_with("ghu_")
            || token.starts_with("ghr_")
            || token.starts_with("ghs_")
    );
}

#[test]
#[ignore]
fn test_get_username_when_authenticated() {
    if !is_installed() || !is_authenticated() {
        return;
    }

    let username = get_username().unwrap();
    assert!(!username.is_empty());
    // Usernames shouldn't contain whitespace
    assert!(!username.contains(char::is_whitespace));
}
