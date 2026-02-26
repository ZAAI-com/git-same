use super::*;
use crate::operations::clone::CloneProgress;

fn sample_repo() -> OwnedRepo {
    OwnedRepo::new("acme", crate::types::Repo::test("rocket", "acme"))
}

#[test]
fn clone_progress_bar_methods_execute_without_panics() {
    let progress = CloneProgressBar::new(2, Verbosity::Verbose);
    let repo = sample_repo();

    progress.on_start(&repo, 1, 2);
    progress.on_complete(&repo, 1, 2);
    progress.on_error(&repo, "network", 2, 2);
    progress.on_skip(&repo, "already cloned", 2, 2);
    progress.finish(1, 1, 0);
}
