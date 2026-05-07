use super::*;

#[test]
fn discovery_progress_bar_methods_execute_without_panics() {
    let progress = DiscoveryProgressBar::new(Verbosity::Verbose);

    progress.on_orgs_discovered(3);
    progress.on_org_started("acme");
    progress.on_org_complete("acme", 7);
    progress.on_personal_repos_started();
    progress.on_personal_repos_complete(2);
    progress.on_error("simulated warning");
    progress.finish();
}
