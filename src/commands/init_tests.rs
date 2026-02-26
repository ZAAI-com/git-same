use super::*;
use crate::cli::InitArgs;
use tempfile::TempDir;

fn quiet_output() -> Output {
    Output::new(crate::output::Verbosity::Quiet, false)
}

#[tokio::test]
async fn test_init_creates_config() {
    let temp = TempDir::new().unwrap();
    let config_path = temp.path().join("config.toml");
    let args = InitArgs {
        force: false,
        path: Some(config_path.clone()),
    };
    let output = quiet_output();

    let result = run(&args, &output).await;
    assert!(result.is_ok());
    assert!(config_path.exists());
    let content = std::fs::read_to_string(&config_path).unwrap();
    assert!(!content.is_empty());
}

#[tokio::test]
async fn test_init_creates_config_dir() {
    let temp = TempDir::new().unwrap();
    let config_path = temp.path().join("git-same/config.toml");
    let args = InitArgs {
        force: false,
        path: Some(config_path.clone()),
    };
    let output = quiet_output();

    let result = run(&args, &output).await;
    assert!(result.is_ok());

    let config_dir = temp.path().join("git-same");
    assert!(config_dir.exists());
    assert!(config_dir.is_dir());
}

#[tokio::test]
async fn test_init_fails_if_exists_without_force() {
    let temp = TempDir::new().unwrap();
    let config_path = temp.path().join("config.toml");
    std::fs::write(&config_path, "existing").unwrap();

    let args = InitArgs {
        force: false,
        path: Some(config_path),
    };
    let output = quiet_output();

    let result = run(&args, &output).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_init_overwrites_with_force() {
    let temp = TempDir::new().unwrap();
    let config_path = temp.path().join("config.toml");
    std::fs::write(&config_path, "old content").unwrap();

    let args = InitArgs {
        force: true,
        path: Some(config_path.clone()),
    };
    let output = quiet_output();

    let result = run(&args, &output).await;
    assert!(result.is_ok());
    let content = std::fs::read_to_string(&config_path).unwrap();
    assert_ne!(content, "old content");
}

#[tokio::test]
async fn test_init_creates_parent_dirs() {
    let temp = TempDir::new().unwrap();
    let config_path = temp.path().join("nested/deep/config.toml");
    let args = InitArgs {
        force: false,
        path: Some(config_path.clone()),
    };
    let output = quiet_output();

    let result = run(&args, &output).await;
    assert!(result.is_ok());
    assert!(config_path.exists());
}

#[test]
fn test_display_check_results_no_panic() {
    let results = vec![
        CheckResult {
            name: "Git".to_string(),
            passed: true,
            message: "git 2.43.0".to_string(),
            suggestion: None,
            critical: true,
        },
        CheckResult {
            name: "SSH".to_string(),
            passed: false,
            message: "no keys".to_string(),
            suggestion: Some("Generate a key".to_string()),
            critical: false,
        },
    ];
    let output = quiet_output();
    display_check_results(&results, &output);
}
