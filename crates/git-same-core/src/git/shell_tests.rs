use super::*;
use std::ffi::OsStr;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

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
    let status = git.parse_status_output("## main...origin/main");
    assert!(!status.is_uncommitted);
    assert!(!status.has_untracked);
    assert_eq!(status.branch, "main");
}

#[test]
fn test_parse_status_modified() {
    let git = ShellGit::new();
    let status = git.parse_status_output("## main\n M src/main.rs");
    assert!(status.is_uncommitted);
    assert!(!status.has_untracked);
    assert_eq!(status.staged_count, 0);
    assert_eq!(status.unstaged_count, 1);
}

#[test]
fn test_parse_status_untracked() {
    let git = ShellGit::new();
    let status = git.parse_status_output("## main\n?? newfile.txt");
    assert!(!status.is_uncommitted);
    assert!(status.has_untracked);
}

#[test]
fn test_parse_status_mixed() {
    let git = ShellGit::new();
    let output = "## feature [ahead 1, behind 2]\n M src/main.rs\n?? newfile.txt\nA  staged.rs";
    let status = git.parse_status_output(output);
    assert!(status.is_uncommitted);
    assert!(status.has_untracked);
    assert_eq!(status.branch, "feature");
    assert_eq!(status.ahead, 1);
    assert_eq!(status.behind, 2);
    assert_eq!(status.staged_count, 1);
    assert_eq!(status.unstaged_count, 1);
    assert_eq!(status.untracked_count, 1);
}

#[test]
fn test_parse_status_header_is_not_a_change() {
    // Counted as a file line, "##" would read as one staged and one
    // unstaged change and every clean repo would look dirty.
    let git = ShellGit::new();
    let cases = [
        ("## main", "main", 0, 0),
        ("## main...origin/main", "main", 0, 0),
        ("## feature...origin/feature [ahead 3]", "feature", 3, 0),
        ("## main...origin/main [behind 5]", "main", 0, 5),
        (
            "## develop...origin/develop [ahead 2, behind 7]",
            "develop",
            2,
            7,
        ),
    ];
    for (header, branch, ahead, behind) in cases {
        let status = git.parse_status_output(header);
        assert_eq!(status.branch, branch, "{header}");
        assert_eq!(status.ahead, ahead, "{header}");
        assert_eq!(status.behind, behind, "{header}");
        assert_eq!(status.staged_count, 0, "{header}");
        assert_eq!(status.unstaged_count, 0, "{header}");
        assert_eq!(status.untracked_count, 0, "{header}");
        assert!(!status.is_uncommitted, "{header}");
        assert!(!status.has_untracked, "{header}");
    }
}

#[test]
fn test_parse_status_counts_each_file_line() {
    let git = ShellGit::new();
    let output = concat!(
        "## main...origin/main [ahead 1]\n",
        "M  staged.rs\n",
        " M unstaged.rs\n",
        "MM both.rs\n",
        "?? a.txt\n",
        "?? b.txt",
    );
    let status = git.parse_status_output(output);
    assert_eq!(status.branch, "main");
    assert_eq!(status.ahead, 1);
    assert_eq!(status.behind, 0);
    assert_eq!(status.staged_count, 2);
    assert_eq!(status.unstaged_count, 2);
    assert_eq!(status.untracked_count, 2);
}

#[test]
fn test_git_command_disables_optional_locks() {
    let cmd = ShellGit::git_command(&["status", "--porcelain"], Some(Path::new("/tmp/repo")));
    let envs: Vec<_> = cmd.get_envs().collect();
    assert!(
        envs.contains(&(OsStr::new("GIT_OPTIONAL_LOCKS"), Some(OsStr::new("0")))),
        "status must not take .git/index.lock: {envs:?}"
    );
    assert!(envs.contains(&(OsStr::new("GIT_TERMINAL_PROMPT"), Some(OsStr::new("0")))));
    let args: Vec<_> = cmd.get_args().collect();
    assert_eq!(args, ["status", "--porcelain"]);
    assert_eq!(cmd.get_current_dir(), Some(Path::new("/tmp/repo")));
}

/// Runs git for test setup, isolated from the user's global and system
/// config (commit signing, hooks, templates) so fixtures behave the same on
/// every machine.
fn git_in(dir: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(dir)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .expect("failed to run git");
    assert!(
        output.status.success(),
        "git {} failed: {}",
        args.join(" "),
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Sets the fixture's identity locally. fsmonitor is pinned off because
/// `ShellGit` itself still reads the user's global config.
fn configure_repo(dir: &Path) {
    git_in(dir, &["config", "user.name", "Git-Same Test"]);
    git_in(dir, &["config", "user.email", "test@example.com"]);
    git_in(dir, &["config", "core.fsmonitor", "false"]);
}

/// Creates a repo on `main` with one committed file, `a.txt`.
fn init_repo_with_commit(dir: &Path) {
    git_in(dir, &["init", "-q", "-b", "main"]);
    configure_repo(dir);
    std::fs::write(dir.join("a.txt"), "hello\n").unwrap();
    git_in(dir, &["add", "a.txt"]);
    git_in(dir, &["commit", "-q", "-m", "initial"]);
}

fn set_mtime(path: &Path, time: SystemTime) {
    std::fs::File::options()
        .write(true)
        .open(path)
        .unwrap()
        .set_modified(time)
        .unwrap();
}

#[test]
fn test_status_does_not_write_index() {
    let dir = tempfile::tempdir().unwrap();
    let repo = dir.path();
    init_repo_with_commit(repo);

    // A tracked file whose mtime no longer matches the index makes a locking
    // `git status` refresh the entry and rewrite .git/index. Pin the index
    // mtime to a known past value so any rewrite is visible even on
    // filesystems with coarse timestamps.
    let index = repo.join(".git").join("index");
    set_mtime(
        &repo.join("a.txt"),
        UNIX_EPOCH + Duration::from_secs(1_500_000_000),
    );
    set_mtime(&index, UNIX_EPOCH + Duration::from_secs(1_600_000_000));
    let before = std::fs::metadata(&index).unwrap().modified().unwrap();

    let status = ShellGit::new().status(repo).unwrap();

    assert!(!status.is_uncommitted, "content is unchanged: {status:?}");
    let after = std::fs::metadata(&index).unwrap().modified().unwrap();
    assert_eq!(before, after, ".git/index was rewritten by status");
    assert!(!repo.join(".git").join("index.lock").exists());
}

#[test]
fn test_status_counts_changes_in_real_repo() {
    let dir = tempfile::tempdir().unwrap();
    let repo = dir.path();
    init_repo_with_commit(repo);

    let clean = ShellGit::new().status(repo).unwrap();
    assert_eq!(clean.branch, "main");
    assert_eq!(
        (
            clean.staged_count,
            clean.unstaged_count,
            clean.untracked_count
        ),
        (0, 0, 0),
        "the ## header must not count as a change"
    );
    assert!(!clean.is_uncommitted);
    assert!(!clean.has_untracked);

    // " M a.txt" sorts first: its leading space must survive so it counts
    // as unstaged, not staged.
    std::fs::write(repo.join("a.txt"), "changed\n").unwrap();
    std::fs::write(repo.join("b.txt"), "staged\n").unwrap();
    git_in(repo, &["add", "b.txt"]);
    std::fs::write(repo.join("c.txt"), "untracked\n").unwrap();

    let dirty = ShellGit::new().status(repo).unwrap();
    assert_eq!(dirty.branch, "main");
    assert_eq!(dirty.staged_count, 1);
    assert_eq!(dirty.unstaged_count, 1);
    assert_eq!(dirty.untracked_count, 1);
    assert!(dirty.is_uncommitted);
    assert!(dirty.has_untracked);
}

#[test]
fn test_status_reports_ahead_and_behind_in_real_repo() {
    let dir = tempfile::tempdir().unwrap();
    let origin = dir.path().join("origin");
    let clone = dir.path().join("clone");
    std::fs::create_dir(&origin).unwrap();
    init_repo_with_commit(&origin);
    git_in(
        dir.path(),
        &[
            "clone",
            "-q",
            origin.to_str().unwrap(),
            clone.to_str().unwrap(),
        ],
    );
    configure_repo(&clone);

    git_in(
        &origin,
        &["commit", "-q", "--allow-empty", "-m", "upstream"],
    );
    git_in(&clone, &["commit", "-q", "--allow-empty", "-m", "local"]);
    git_in(&clone, &["fetch", "-q"]);

    let status = ShellGit::new().status(&clone).unwrap();
    assert_eq!(status.branch, "main");
    assert_eq!(status.ahead, 1);
    assert_eq!(status.behind, 1);
    assert!(!status.is_uncommitted);
    assert_eq!(status.staged_count + status.unstaged_count, 0);
}

// Unit tests for parse_track_info
#[test]
fn test_parse_track_info_empty() {
    let (ahead, behind) = ShellGit::parse_track_info("");
    assert_eq!(ahead, 0);
    assert_eq!(behind, 0);
}

#[test]
fn test_parse_track_info_ahead() {
    let (ahead, behind) = ShellGit::parse_track_info("[ahead 5]");
    assert_eq!(ahead, 5);
    assert_eq!(behind, 0);
}

#[test]
fn test_parse_track_info_behind() {
    let (ahead, behind) = ShellGit::parse_track_info("[behind 3]");
    assert_eq!(ahead, 0);
    assert_eq!(behind, 3);
}

#[test]
fn test_parse_track_info_diverged() {
    let (ahead, behind) = ShellGit::parse_track_info("[ahead 2, behind 7]");
    assert_eq!(ahead, 2);
    assert_eq!(behind, 7);
}

#[test]
fn test_parse_track_info_gone_yields_zeros() {
    // "[gone]" is the for-each-ref signal that the upstream ref no longer
    // exists. parse_track_info doesn't decode it, but list_branches relies
    // on it returning (0, 0) so the higher-level upstream-clearing logic
    // can take over without spurious counts leaking through.
    let (ahead, behind) = ShellGit::parse_track_info("[gone]");
    assert_eq!(ahead, 0);
    assert_eq!(behind, 0);
}

#[test]
fn test_parse_branch_line_synced() {
    let info = ShellGit::parse_branch_line("main\torigin/main\t").unwrap();
    assert_eq!(info.name, "main");
    assert_eq!(info.upstream, Some("origin/main".to_string()));
    assert_eq!(info.ahead, 0);
    assert_eq!(info.behind, 0);
    assert!(info.is_synced);
}

#[test]
fn test_parse_branch_line_no_upstream() {
    let info = ShellGit::parse_branch_line("local-only\t\t").unwrap();
    assert!(info.upstream.is_none());
    assert!(!info.is_synced);
}

#[test]
fn test_parse_branch_line_diverged() {
    let info = ShellGit::parse_branch_line("feature\torigin/feature\t[ahead 2, behind 7]").unwrap();
    assert_eq!(info.upstream, Some("origin/feature".to_string()));
    assert_eq!(info.ahead, 2);
    assert_eq!(info.behind, 7);
    assert!(!info.is_synced);
}

#[test]
fn test_parse_branch_line_gone_upstream_not_synced() {
    // When the upstream ref has been deleted, %(upstream:short) still
    // returns the now-dead name and %(upstream:track) is "[gone]". The
    // branch must not be reported as synced; its commits are local-only.
    let info = ShellGit::parse_branch_line("orphan\torigin/orphan\t[gone]").unwrap();
    assert!(
        info.upstream.is_none(),
        "[gone] upstream should be cleared so the branch isn't presented as tracking a live ref"
    );
    assert!(
        !info.is_synced,
        "branch with deleted upstream must not be reported as synced; Badge::Green would mislead the user into deleting unique commits"
    );
}

#[test]
fn test_parse_branch_line_empty_returns_none() {
    assert!(ShellGit::parse_branch_line("").is_none());
    assert!(ShellGit::parse_branch_line("\torigin/foo\t").is_none());
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

#[test]
#[ignore]
fn test_list_branches_real() {
    let git = ShellGit::new();
    let branches = git.list_branches(Path::new("."));
    assert!(branches.is_ok());
    let branches = branches.unwrap();
    // Should have at least one branch
    assert!(!branches.is_empty());
    // At least one branch should have a name
    assert!(!branches[0].name.is_empty());
}

#[test]
#[ignore]
fn test_list_remotes_real() {
    let git = ShellGit::new();
    let remotes = git.list_remotes(Path::new("."));
    assert!(remotes.is_ok());
    let remotes = remotes.unwrap();
    // Should have at least one remote (origin)
    assert!(!remotes.is_empty());
    assert_eq!(remotes[0].name, "origin");
    assert!(!remotes[0].fetch_url.is_empty());
}

#[test]
#[ignore]
fn test_list_worktrees_real() {
    let git = ShellGit::new();
    let worktrees = git.list_worktrees(Path::new("."));
    assert!(worktrees.is_ok());
    let worktrees = worktrees.unwrap();
    // Should have at least the main worktree
    assert!(!worktrees.is_empty());
    assert!(worktrees[0].path.exists());
}

#[test]
#[ignore]
fn test_commit_count_real() {
    let git = ShellGit::new();
    let count = git.commit_count(Path::new("."));
    assert!(count.is_ok());
    // Should have at least 1 commit
    assert!(count.unwrap() > 0);
}

#[test]
#[ignore]
fn test_stash_count_real() {
    let git = ShellGit::new();
    let count = git.stash_count(Path::new("."));
    assert!(count.is_ok());
    // Just check it doesn't error; count could be 0
}

#[test]
#[ignore]
fn test_list_ignored_files_real() {
    let git = ShellGit::new();
    let files = git.list_ignored_files(Path::new("."));
    assert!(files.is_ok());
    // Just check it doesn't error; could be empty
}
