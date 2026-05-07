use super::*;
use crate::operations::sync::SyncProgress;

fn sample_repo() -> OwnedRepo {
    OwnedRepo::new("acme", crate::types::Repo::test("rocket", "acme"))
}

#[test]
fn sync_progress_bar_methods_execute_without_panics() {
    let progress = SyncProgressBar::new(3, Verbosity::Verbose, "Fetch");
    let repo = sample_repo();
    let temp_dir = std::env::temp_dir();

    progress.on_start(&repo, temp_dir.as_path(), 1, 3);
    progress.on_fetch_complete(
        &repo,
        &FetchResult {
            updated: true,
            new_commits: Some(4),
        },
        1,
        3,
    );
    progress.on_pull_complete(
        &repo,
        &PullResult {
            success: true,
            updated: true,
            fast_forward: true,
            error: None,
        },
        2,
        3,
    );
    progress.on_error(&repo, "sync failed", 3, 3);
    progress.on_skip(&repo, "dirty tree", 3, 3);
    progress.finish(2, 1, 0);
}
