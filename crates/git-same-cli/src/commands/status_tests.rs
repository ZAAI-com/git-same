use super::*;
use git_same_core::output::Verbosity;

fn quiet_output() -> Output {
    Output::new(Verbosity::Quiet, false)
}

#[tokio::test]
async fn test_status_no_workspaces() {
    let args = StatusArgs {
        workspace: Some("nonexistent".to_string()),
        uncommitted: false,
        behind: false,
        detailed: false,
        org: vec![],
    };
    let config = Config::default();
    let output = quiet_output();

    let result = run(&args, &config, &output).await;
    let err = result.expect_err("nonexistent workspace should return an error");
    assert!(
        err.to_string().contains("not found")
            || err.to_string().contains("No workspace config found")
            || err.to_string().contains("Configuration error"),
        "unexpected error: {}",
        err
    );
}
