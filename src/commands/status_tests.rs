use super::*;
use crate::output::Verbosity;

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
    assert!(result.is_err());
}
