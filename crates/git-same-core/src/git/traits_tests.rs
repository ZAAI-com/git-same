use super::*;

#[test]
fn test_clone_options_builder() {
    let options = CloneOptions::new()
        .with_depth(1)
        .with_branch("develop")
        .with_submodules();

    assert_eq!(options.depth, 1);
    assert_eq!(options.branch, Some("develop".to_string()));
    assert!(options.recurse_submodules);
}

#[test]
fn test_clone_options_default() {
    let options = CloneOptions::default();
    assert_eq!(options.depth, 0);
    assert!(options.branch.is_none());
    assert!(!options.recurse_submodules);
}

#[test]
fn test_repo_status_clean_and_synced() {
    let status = RepoStatus {
        branch: "main".to_string(),
        is_uncommitted: false,
        ahead: 0,
        behind: 0,
        has_untracked: false,
        staged_count: 0,
        unstaged_count: 0,
        untracked_count: 0,
    };
    assert!(status.is_clean_and_synced());

    let uncommitted_status = RepoStatus {
        is_uncommitted: true,
        ..status.clone()
    };
    assert!(!uncommitted_status.is_clean_and_synced());

    let ahead = RepoStatus {
        ahead: 1,
        ..status.clone()
    };
    assert!(!ahead.is_clean_and_synced());
}

#[test]
fn test_repo_status_can_fast_forward() {
    let status = RepoStatus {
        branch: "main".to_string(),
        is_uncommitted: false,
        ahead: 0,
        behind: 3,
        has_untracked: false,
        staged_count: 0,
        unstaged_count: 0,
        untracked_count: 0,
    };
    assert!(status.can_fast_forward());

    let uncommitted_status = RepoStatus {
        is_uncommitted: true,
        ..status.clone()
    };
    assert!(!uncommitted_status.can_fast_forward());

    let diverged = RepoStatus {
        ahead: 1,
        behind: 3,
        ..status.clone()
    };
    assert!(!diverged.can_fast_forward());
}

#[test]
fn test_branch_info_synced() {
    let branch = BranchInfo {
        name: "main".to_string(),
        upstream: Some("origin/main".to_string()),
        ahead: 0,
        behind: 0,
        is_synced: true,
    };
    assert!(branch.is_synced);
    assert_eq!(branch.name, "main");
}

#[test]
fn test_branch_info_no_upstream() {
    let branch = BranchInfo {
        name: "local-only".to_string(),
        upstream: None,
        ahead: 0,
        behind: 0,
        is_synced: false,
    };
    assert!(!branch.is_synced);
}

#[test]
fn test_remote_info() {
    let remote = RemoteInfo {
        name: "origin".to_string(),
        fetch_url: "git@github.com:user/repo.git".to_string(),
        push_url: "git@github.com:user/repo.git".to_string(),
    };
    assert_eq!(remote.name, "origin");
    assert_eq!(remote.fetch_url, remote.push_url);
}

#[test]
fn test_worktree_info() {
    let wt = WorktreeInfo {
        path: Path::new("/tmp/worktree").to_path_buf(),
        branch: Some("feature".to_string()),
        is_bare: false,
        is_detached: false,
    };
    assert_eq!(wt.branch.as_deref(), Some("feature"));
    assert!(!wt.is_bare);
    assert!(!wt.is_detached);
}

#[test]
fn test_worktree_info_detached() {
    let wt = WorktreeInfo {
        path: Path::new("/tmp/worktree").to_path_buf(),
        branch: None,
        is_bare: false,
        is_detached: true,
    };
    assert!(wt.branch.is_none());
    assert!(wt.is_detached);
}

mod mock_tests {
    use super::mock::*;
    use super::*;

    #[test]
    fn test_mock_clone_success() {
        let mock = MockGit::new();
        let result = mock.clone_repo(
            "git@github.com:user/repo.git",
            Path::new("/tmp/repo"),
            &CloneOptions::default(),
        );
        assert!(result.is_ok());

        let log = mock.call_log();
        assert_eq!(log.clones.len(), 1);
        assert_eq!(log.clones[0].0, "git@github.com:user/repo.git");
    }

    #[test]
    fn test_mock_clone_failure() {
        let mut mock = MockGit::new();
        mock.fail_clones(Some("permission denied".to_string()));

        let result = mock.clone_repo(
            "git@github.com:user/repo.git",
            Path::new("/tmp/repo"),
            &CloneOptions::default(),
        );
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert!(err.to_string().contains("permission denied"));
    }

    #[test]
    fn test_mock_fetch() {
        let config = MockConfig {
            fetch_has_updates: true,
            ..Default::default()
        };
        let mock = MockGit::with_config(config);

        let result = mock.fetch(Path::new("/tmp/repo")).unwrap();
        assert!(result.updated);
        assert_eq!(result.new_commits, Some(3));
    }

    #[test]
    fn test_mock_pull() {
        let mock = MockGit::new();
        let result = mock.pull(Path::new("/tmp/repo")).unwrap();
        assert!(result.success);
        assert!(result.fast_forward);
    }

    #[test]
    fn test_mock_status_default() {
        let mock = MockGit::new();
        let status = mock.status(Path::new("/tmp/repo")).unwrap();
        assert_eq!(status.branch, "main");
        assert!(!status.is_uncommitted);
    }

    #[test]
    fn test_mock_status_custom() {
        let mut mock = MockGit::new();
        mock.set_status(
            "/tmp/repo",
            RepoStatus {
                branch: "feature".to_string(),
                is_uncommitted: true,
                ahead: 2,
                behind: 0,
                has_untracked: true,
                staged_count: 0,
                unstaged_count: 0,
                untracked_count: 0,
            },
        );

        let status = mock.status(Path::new("/tmp/repo")).unwrap();
        assert_eq!(status.branch, "feature");
        assert!(status.is_uncommitted);
        assert_eq!(status.ahead, 2);
    }

    #[test]
    fn test_mock_is_repo() {
        let mut mock = MockGit::new();
        mock.add_repo("/tmp/repo");

        assert!(mock.is_repo(Path::new("/tmp/repo")));
        assert!(!mock.is_repo(Path::new("/tmp/not-a-repo")));
    }

    #[test]
    fn test_mock_list_branches() {
        let mock = MockGit::new();
        let branches = mock.list_branches(Path::new("/tmp/repo")).unwrap();
        assert_eq!(branches.len(), 1);
        assert_eq!(branches[0].name, "main");
        assert!(branches[0].is_synced);
    }

    #[test]
    fn test_mock_list_remotes() {
        let mock = MockGit::new();
        let remotes = mock.list_remotes(Path::new("/tmp/repo")).unwrap();
        assert_eq!(remotes.len(), 1);
        assert_eq!(remotes[0].name, "origin");
    }

    #[test]
    fn test_mock_list_worktrees() {
        let mock = MockGit::new();
        let worktrees = mock.list_worktrees(Path::new("/tmp/repo")).unwrap();
        assert!(worktrees.is_empty());
    }

    #[test]
    fn test_mock_commit_count() {
        let mock = MockGit::new();
        let count = mock.commit_count(Path::new("/tmp/repo")).unwrap();
        assert_eq!(count, 42);
    }

    #[test]
    fn test_mock_stash_count() {
        let mock = MockGit::new();
        let count = mock.stash_count(Path::new("/tmp/repo")).unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_mock_list_ignored_files() {
        let mock = MockGit::new();
        let files = mock.list_ignored_files(Path::new("/tmp/repo")).unwrap();
        assert!(files.is_empty());
    }

    #[test]
    fn test_mock_call_log_tracking() {
        let mock = MockGit::new();

        let _ = mock.clone_repo("url1", Path::new("/path1"), &CloneOptions::default());
        let _ = mock.fetch(Path::new("/path2"));
        let _ = mock.pull(Path::new("/path3"));
        let _ = mock.status(Path::new("/path4"));

        let log = mock.call_log();
        assert_eq!(log.clones.len(), 1);
        assert_eq!(log.fetches.len(), 1);
        assert_eq!(log.pulls.len(), 1);
        assert_eq!(log.status_checks.len(), 1);
    }
}
