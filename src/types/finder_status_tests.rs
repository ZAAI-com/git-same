use super::*;

#[test]
fn test_compute_badge_green() {
    let badge = compute_badge(0, 0, 0, 0, true, true, false);
    assert_eq!(badge, Badge::Green);
}

#[test]
fn test_compute_badge_red_staged() {
    let badge = compute_badge(1, 0, 0, 0, true, true, false);
    assert_eq!(badge, Badge::Red);
}

#[test]
fn test_compute_badge_red_unstaged() {
    let badge = compute_badge(0, 2, 0, 0, true, true, false);
    assert_eq!(badge, Badge::Red);
}

#[test]
fn test_compute_badge_red_untracked() {
    let badge = compute_badge(0, 0, 3, 0, true, true, false);
    assert_eq!(badge, Badge::Red);
}

#[test]
fn test_compute_badge_red_ahead() {
    let badge = compute_badge(0, 0, 0, 1, true, true, false);
    assert_eq!(badge, Badge::Red);
}

#[test]
fn test_compute_badge_orange_branches_not_synced() {
    let badge = compute_badge(0, 0, 0, 0, false, true, false);
    assert_eq!(badge, Badge::Orange);
}

#[test]
fn test_compute_badge_orange_worktrees_not_synced() {
    let badge = compute_badge(0, 0, 0, 0, true, false, false);
    assert_eq!(badge, Badge::Orange);
}

#[test]
fn test_compute_badge_blue_important_ignored() {
    let badge = compute_badge(0, 0, 0, 0, true, true, true);
    assert_eq!(badge, Badge::Blue);
}

#[test]
fn test_compute_badge_priority_red_over_orange() {
    // Even if branches not synced, staged files = Red
    let badge = compute_badge(1, 0, 0, 0, false, false, false);
    assert_eq!(badge, Badge::Red);
}

#[test]
fn test_compute_badge_priority_orange_over_blue() {
    // Branches not synced + important ignored = Orange (not Blue)
    let badge = compute_badge(0, 0, 0, 0, false, true, true);
    assert_eq!(badge, Badge::Orange);
}

#[test]
fn test_matches_important_pattern_env() {
    let patterns = DEFAULT_IMPORTANT_IGNORED_PATTERNS;
    assert!(matches_important_pattern(".env", patterns));
    assert!(matches_important_pattern(".env.local", patterns));
    assert!(matches_important_pattern(".env.production", patterns));
    assert!(matches_important_pattern("subdir/.env", patterns));
}

#[test]
fn test_matches_important_pattern_keys() {
    let patterns = DEFAULT_IMPORTANT_IGNORED_PATTERNS;
    assert!(matches_important_pattern("server.key", patterns));
    assert!(matches_important_pattern("cert.pem", patterns));
    assert!(matches_important_pattern("signing.p12", patterns));
}

#[test]
fn test_matches_important_pattern_credentials() {
    let patterns = DEFAULT_IMPORTANT_IGNORED_PATTERNS;
    assert!(matches_important_pattern("credentials.json", patterns));
    assert!(matches_important_pattern("secrets.yaml", patterns));
    assert!(matches_important_pattern(
        "service-account-prod.json",
        patterns
    ));
}

#[test]
fn test_matches_important_pattern_no_match() {
    let patterns = DEFAULT_IMPORTANT_IGNORED_PATTERNS;
    assert!(!matches_important_pattern("main.rs", patterns));
    assert!(!matches_important_pattern(
        "node_modules/lodash/index.js",
        patterns
    ));
    assert!(!matches_important_pattern(
        "target/debug/git-same",
        patterns
    ));
    assert!(!matches_important_pattern("README.md", patterns));
}

#[test]
fn test_glob_match_exact() {
    assert!(simple_glob_match(".env", ".env"));
    assert!(!simple_glob_match(".env", ".envx"));
}

#[test]
fn test_glob_match_star() {
    assert!(simple_glob_match("*.key", "server.key"));
    assert!(simple_glob_match("*.key", ".key"));
    assert!(!simple_glob_match("*.key", "server.pem"));
}

#[test]
fn test_glob_match_dot_star() {
    assert!(simple_glob_match(".env.*", ".env.local"));
    assert!(simple_glob_match(".env.*", ".env.production"));
    assert!(!simple_glob_match(".env.*", ".env"));
}

#[test]
fn test_glob_match_question_mark() {
    assert!(simple_glob_match("?.key", "a.key"));
    assert!(!simple_glob_match("?.key", "ab.key"));
}

#[test]
fn test_finder_status_serialization() {
    let status = FinderStatus::new(12345, "2026-04-04T10:30:00Z".to_string());
    let json = serde_json::to_string_pretty(&status).unwrap();
    assert!(json.contains("\"version\": 1"));
    assert!(json.contains("\"daemon_pid\": 12345"));

    // Round-trip
    let parsed: FinderStatus = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, status);
}

#[test]
fn test_finder_repo_status_serialization() {
    let repo = FinderRepoStatus {
        path: PathBuf::from("/repos/org/repo"),
        workspace: Some("github".to_string()),
        org: Some("org".to_string()),
        badge: Badge::Green,
        current_branch: "main".to_string(),
        default_branch: Some("main".to_string()),
        commit_count: 847,
        staged_count: 0,
        unstaged_count: 0,
        untracked_count: 0,
        ahead: 0,
        behind: 0,
        stash_count: 0,
        has_important_ignored_files: false,
        important_ignored_files: Vec::new(),
        branches: vec![FinderBranchInfo {
            name: "main".to_string(),
            upstream: Some("origin/main".to_string()),
            ahead: 0,
            behind: 0,
            synced: true,
        }],
        all_branches_synced: true,
        remotes: vec![FinderRemoteInfo {
            name: "origin".to_string(),
            url: "git@github.com:org/repo.git".to_string(),
        }],
        worktrees: Vec::new(),
        all_worktrees_synced: true,
    };

    let json = serde_json::to_string(&repo).unwrap();
    assert!(json.contains("\"badge\":\"green\""));

    let parsed: FinderRepoStatus = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, repo);
}

#[test]
fn test_badge_serialization() {
    assert_eq!(serde_json::to_string(&Badge::Green).unwrap(), "\"green\"");
    assert_eq!(serde_json::to_string(&Badge::Blue).unwrap(), "\"blue\"");
    assert_eq!(serde_json::to_string(&Badge::Orange).unwrap(), "\"orange\"");
    assert_eq!(serde_json::to_string(&Badge::Red).unwrap(), "\"red\"");
}
