use super::*;
use crate::cli::RefreshArgs;
use git_same_core::output::{Output, Verbosity};

#[tokio::test]
async fn refresh_with_no_monitor_returns_error_on_unix() {
    // With no monitor listening on the socket, the command must surface an
    // error (unlike the post-sync/post-reset nudges, which stay silent).
    let args = RefreshArgs { path: None };
    let cfg = Config::default();
    let output = Output::new(Verbosity::Quiet, false);

    #[cfg(unix)]
    {
        let res = run(&args, &cfg, &output).await;
        assert!(res.is_err(), "expected error when monitor is not running");
    }
    #[cfg(not(unix))]
    {
        let res = run(&args, &cfg, &output).await;
        assert!(res.is_ok(), "non-unix fallback should succeed");
    }
}
