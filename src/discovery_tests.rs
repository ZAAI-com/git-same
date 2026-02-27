use super::*;
use crate::git::MockGit;
use crate::types::Repo;
use tempfile::TempDir;

fn test_repo(name: &str, owner: &str) -> OwnedRepo {
    OwnedRepo::new(owner, Repo::test(name, owner))
}

#[test]
fn test_orchestrator_creation() {
    let filters = FilterOptions::default();
    let orchestrator = DiscoveryOrchestrator::new(filters, "{org}/{repo}".to_string());
    assert_eq!(orchestrator.structure, "{org}/{repo}");
}

#[test]
fn test_compute_path_simple() {
    let filters = FilterOptions::default();
    let orchestrator = DiscoveryOrchestrator::new(filters, "{org}/{repo}".to_string());

    let repo = test_repo("my-repo", "my-org");
    let path = orchestrator.compute_path(Path::new("/base"), &repo, "github");

    assert_eq!(path, PathBuf::from("/base/my-org/my-repo"));
}

#[test]
fn test_compute_path_with_provider() {
    let filters = FilterOptions::default();
    let orchestrator = DiscoveryOrchestrator::new(filters, "{provider}/{org}/{repo}".to_string());

    let repo = test_repo("my-repo", "my-org");
    let path = orchestrator.compute_path(Path::new("/base"), &repo, "github");

    assert_eq!(path, PathBuf::from("/base/github/my-org/my-repo"));
}

#[test]
fn test_plan_clone_new_repos() {
    let filters = FilterOptions::default();
    let orchestrator = DiscoveryOrchestrator::new(filters, "{org}/{repo}".to_string());
    let git = MockGit::new();

    let repos = vec![test_repo("repo1", "org"), test_repo("repo2", "org")];

    let plan = orchestrator.plan_clone(Path::new("/nonexistent"), repos, "github", &git);

    assert_eq!(plan.to_clone.len(), 2);
    assert_eq!(plan.to_sync.len(), 0);
    assert_eq!(plan.skipped.len(), 0);
}

#[test]
fn test_plan_clone_existing_repos() {
    let temp = TempDir::new().unwrap();
    let repo_path = temp.path().join("org/repo");
    std::fs::create_dir_all(&repo_path).unwrap();

    let mut git = MockGit::new();
    git.add_repo(repo_path.to_string_lossy().to_string());

    let filters = FilterOptions::default();
    let orchestrator = DiscoveryOrchestrator::new(filters, "{org}/{repo}".to_string());

    let repos = vec![test_repo("repo", "org")];
    let plan = orchestrator.plan_clone(temp.path(), repos, "github", &git);

    assert_eq!(plan.to_clone.len(), 0);
    assert_eq!(plan.to_sync.len(), 1);
    assert_eq!(plan.skipped.len(), 0);
}

#[test]
fn test_plan_clone_non_repo_dir() {
    let temp = TempDir::new().unwrap();
    let repo_path = temp.path().join("org/repo");
    std::fs::create_dir_all(&repo_path).unwrap();

    let git = MockGit::new(); // Not marked as a repo

    let filters = FilterOptions::default();
    let orchestrator = DiscoveryOrchestrator::new(filters, "{org}/{repo}".to_string());

    let repos = vec![test_repo("repo", "org")];
    let plan = orchestrator.plan_clone(temp.path(), repos, "github", &git);

    assert_eq!(plan.to_clone.len(), 0);
    assert_eq!(plan.to_sync.len(), 0);
    assert_eq!(plan.skipped.len(), 1);
}

#[test]
fn test_plan_sync() {
    let temp = TempDir::new().unwrap();
    let repo_path = temp.path().join("org/repo");
    std::fs::create_dir_all(&repo_path).unwrap();

    let mut git = MockGit::new();
    git.add_repo(repo_path.to_string_lossy().to_string());

    let filters = FilterOptions::default();
    let orchestrator = DiscoveryOrchestrator::new(filters, "{org}/{repo}".to_string());

    let repos = vec![test_repo("repo", "org")];
    let (to_sync, skipped) = orchestrator.plan_sync(temp.path(), repos, "github", &git, false);

    assert_eq!(to_sync.len(), 1);
    assert_eq!(skipped.len(), 0);
}

#[test]
fn test_plan_sync_not_cloned() {
    let filters = FilterOptions::default();
    let orchestrator = DiscoveryOrchestrator::new(filters, "{org}/{repo}".to_string());
    let git = MockGit::new();

    let repos = vec![test_repo("repo", "org")];
    let (to_sync, skipped) =
        orchestrator.plan_sync(Path::new("/nonexistent"), repos, "github", &git, false);

    assert_eq!(to_sync.len(), 0);
    assert_eq!(skipped.len(), 1);
    assert!(skipped[0].1.contains("not cloned"));
}

#[cfg(unix)]
#[test]
fn test_scan_local_ignores_symlinked_directories() {
    use std::os::unix::fs::symlink;

    let temp = TempDir::new().unwrap();
    let repo_path = temp.path().join("org/repo");
    std::fs::create_dir_all(&repo_path).unwrap();
    symlink(temp.path().join("org"), temp.path().join("org-link")).unwrap();

    let mut git = MockGit::new();
    git.add_repo(
        std::fs::canonicalize(&repo_path)
            .unwrap()
            .to_string_lossy()
            .to_string(),
    );

    let orchestrator = DiscoveryOrchestrator::new(FilterOptions::default(), "{org}/{repo}".into());
    let repos = orchestrator.scan_local(temp.path(), &git);

    assert_eq!(repos.len(), 1);
    assert_eq!(repos[0].1, "org");
    assert_eq!(repos[0].2, "repo");
}

#[test]
fn test_merge_repos() {
    let repos1 = vec![test_repo("repo1", "org1")];
    let repos2 = vec![test_repo("repo2", "org2")];

    let merged = merge_repos(vec![
        ("github".to_string(), repos1),
        ("gitlab".to_string(), repos2),
    ]);

    assert_eq!(merged.len(), 2);
    assert_eq!(merged[0].0, "github");
    assert_eq!(merged[1].0, "gitlab");
}

#[test]
fn test_deduplicate_repos() {
    let repo1 = test_repo("repo", "org");
    let repo2 = test_repo("repo", "org"); // Duplicate

    let repos = vec![("github".to_string(), repo1), ("gitlab".to_string(), repo2)];

    let deduped = deduplicate_repos(repos);
    assert_eq!(deduped.len(), 1);
    assert_eq!(deduped[0].0, "github"); // First one wins
}

#[test]
fn test_to_discovery_options() {
    let filters = FilterOptions {
        include_archived: true,
        include_forks: false,
        orgs: vec!["org1".to_string(), "org2".to_string()],
        exclude_repos: vec!["org/skip-this".to_string()],
    };

    let orchestrator = DiscoveryOrchestrator::new(filters.clone(), "{org}/{repo}".to_string());
    let options = orchestrator.to_discovery_options();

    assert!(options.include_archived);
    assert!(!options.include_forks);
    assert_eq!(options.org_filter, vec!["org1", "org2"]);
}
