use super::*;

#[test]
fn test_check_git_installed_runs() {
    let result = check_git_installed();
    // Just verify it runs without panic; actual result depends on environment
    assert_eq!(result.name, "Git");
    assert!(result.critical);
}

#[test]
fn test_check_gh_installed_runs() {
    let result = check_gh_installed();
    assert_eq!(result.name, "GitHub CLI");
    assert!(result.critical);
}

#[test]
fn test_check_result_fields() {
    let result = CheckResult {
        name: "Test".to_string(),
        passed: true,
        message: "ok".to_string(),
        suggestion: None,
        critical: false,
    };
    assert!(result.passed);
    assert!(result.suggestion.is_none());
}

#[tokio::test]
async fn test_check_requirements_returns_all_checks() {
    let results = check_requirements().await;
    assert_eq!(results.len(), 4);
    assert_eq!(results[0].name, "Git");
    assert_eq!(results[1].name, "GitHub CLI");
    assert_eq!(results[2].name, "GitHub Auth");
    assert_eq!(results[3].name, "SSH GitHub");
}
