use super::*;

#[test]
fn test_shell_git_creation() {
    let _git = ShellGit::new();
    // ShellGit is a zero-sized type with no fields
}

#[test]
fn test_parse_branch_info_simple() {
    let git = ShellGit::new();
    let (branch, ahead, behind) = git.parse_branch_info("## main");
    assert_eq!(branch, "main");
    assert_eq!(ahead, 0);
    assert_eq!(behind, 0);
}

#[test]
fn test_parse_branch_info_with_tracking() {
    let git = ShellGit::new();
    let (branch, ahead, behind) = git.parse_branch_info("## main...origin/main");
    assert_eq!(branch, "main");
    assert_eq!(ahead, 0);
    assert_eq!(behind, 0);
}

#[test]
fn test_parse_branch_info_ahead() {
    let git = ShellGit::new();
    let (branch, ahead, behind) = git.parse_branch_info("## feature...origin/feature [ahead 3]");
    assert_eq!(branch, "feature");
    assert_eq!(ahead, 3);
    assert_eq!(behind, 0);
}

#[test]
fn test_parse_branch_info_behind() {
    let git = ShellGit::new();
    let (branch, ahead, behind) = git.parse_branch_info("## main...origin/main [behind 5]");
    assert_eq!(branch, "main");
    assert_eq!(ahead, 0);
    assert_eq!(behind, 5);
}

#[test]
fn test_parse_branch_info_diverged() {
    let git = ShellGit::new();
    let (branch, ahead, behind) =
        git.parse_branch_info("## develop...origin/develop [ahead 2, behind 7]");
    assert_eq!(branch, "develop");
    assert_eq!(ahead, 2);
    assert_eq!(behind, 7);
}

#[test]
fn test_parse_status_clean() {
    let git = ShellGit::new();
    let status = git.parse_status_output("", "## main...origin/main");
    assert!(!status.is_uncommitted);
    assert!(!status.has_untracked);
    assert_eq!(status.branch, "main");
}

#[test]
fn test_parse_status_modified() {
    let git = ShellGit::new();
    let status = git.parse_status_output(" M src/main.rs", "## main");
    assert!(status.is_uncommitted);
    assert!(!status.has_untracked);
}

#[test]
fn test_parse_status_untracked() {
    let git = ShellGit::new();
    let status = git.parse_status_output("?? newfile.txt", "## main");
    assert!(!status.is_uncommitted);
    assert!(status.has_untracked);
}

#[test]
fn test_parse_status_mixed() {
    let git = ShellGit::new();
    let output = " M src/main.rs\n?? newfile.txt\nA  staged.rs";
    let status = git.parse_status_output(output, "## feature [ahead 1, behind 2]");
    assert!(status.is_uncommitted);
    assert!(status.has_untracked);
    assert_eq!(status.branch, "feature");
    assert_eq!(status.ahead, 1);
    assert_eq!(status.behind, 2);
}

// Integration tests that require actual git repo
#[test]
#[ignore] // Run with: cargo test -- --ignored
fn test_is_repo_real() {
    let git = ShellGit::new();
    // Current directory should be a git repo
    assert!(git.is_repo(Path::new(".")));
    // Root is not a git repo
    assert!(!git.is_repo(Path::new("/")));
}

#[test]
#[ignore]
fn test_current_branch_real() {
    let git = ShellGit::new();
    let branch = git.current_branch(Path::new("."));
    assert!(branch.is_ok());
    // Should return some branch name
    assert!(!branch.unwrap().is_empty());
}

#[test]
#[ignore]
fn test_status_real() {
    let git = ShellGit::new();
    let status = git.status(Path::new("."));
    assert!(status.is_ok());
    let status = status.unwrap();
    // Should have a branch
    assert!(!status.branch.is_empty());
}
