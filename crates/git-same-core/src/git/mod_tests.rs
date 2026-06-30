use super::*;
use std::path::Path;

#[test]
fn reexports_are_accessible() {
    let _git = ShellGit::new();

    let options = CloneOptions::new().with_depth(1).with_branch("main");
    assert_eq!(options.depth, 1);
    assert_eq!(options.branch.as_deref(), Some("main"));
}

#[test]
fn mock_git_reexport_behaves_as_expected() {
    let mock = MockGit::new();
    let status = mock.status(Path::new("/tmp/nonexistent")).unwrap();

    assert_eq!(status.branch, "main");
    assert!(!status.is_uncommitted);
}
