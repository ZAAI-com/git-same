//! Integration tests for git-same CLI.
//!
//! These tests verify the CLI behavior as a whole.

use std::path::{Path, PathBuf};
use std::process::Command;

fn git_same_binary() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("target/debug/git-same");
    path
}

fn command_with_temp_env(home: &Path) -> Command {
    let mut cmd = Command::new(git_same_binary());
    cmd.env("HOME", home)
        .env("XDG_CONFIG_HOME", home.join(".config"))
        .env("XDG_CACHE_HOME", home.join(".cache"))
        .env("NO_COLOR", "1");
    cmd
}

fn run_cli_with_env(home: &Path, args: &[&str]) -> std::process::Output {
    command_with_temp_env(home)
        .args(args)
        .output()
        .expect("Failed to execute git-same")
}

fn assert_banner_branding(stdout: &str) {
    let description = env!("CARGO_PKG_DESCRIPTION");
    assert!(
        stdout.contains("██████╗ ██╗████████╗"),
        "Expected ASCII logo in stdout, got:\n{}",
        stdout
    );
    assert!(
        stdout.contains(description),
        "Expected subheadline '{}' in stdout, got:\n{}",
        description,
        stdout
    );
    assert!(
        !stdout.contains(&format!("{description}  Version")),
        "Unexpected legacy version suffix in subheadline, got:\n{}",
        stdout
    );
}

#[test]
fn test_help_command() {
    let output = Command::new(git_same_binary())
        .arg("--help")
        .output()
        .expect("Failed to execute git-same");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains(env!("CARGO_PKG_DESCRIPTION")));
    assert!(stdout.contains("init"));
    assert!(stdout.contains("setup"));
    assert!(stdout.contains("sync"));
    assert!(stdout.contains("status"));
    assert!(stdout.contains("workspace"));
    assert!(stdout.contains("reset"));
}

#[test]
fn test_reset_help() {
    let output = Command::new(git_same_binary())
        .args(["reset", "--help"])
        .output()
        .expect("Failed to execute git-same");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Reset"));
    assert!(stdout.contains("--force"));
}

#[test]
fn test_version_command() {
    let output = Command::new(git_same_binary())
        .arg("--version")
        .output()
        .expect("Failed to execute git-same");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("git-same"));
}

#[test]
fn test_clone_subcommand_removed() {
    let output = Command::new(git_same_binary())
        .arg("clone")
        .output()
        .expect("Failed to execute git-same");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("unrecognized subcommand") || stderr.contains("unexpected argument"),
        "Expected unknown subcommand error, got: {}",
        stderr
    );
}

#[test]
fn test_fetch_subcommand_removed() {
    let output = Command::new(git_same_binary())
        .arg("fetch")
        .output()
        .expect("Failed to execute git-same");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("unrecognized subcommand") || stderr.contains("unexpected argument"),
        "Expected unknown subcommand error, got: {}",
        stderr
    );
}

#[test]
fn test_pull_subcommand_removed() {
    let output = Command::new(git_same_binary())
        .arg("pull")
        .output()
        .expect("Failed to execute git-same");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("unrecognized subcommand") || stderr.contains("unexpected argument"),
        "Expected unknown subcommand error, got: {}",
        stderr
    );
}

#[test]
fn test_status_help() {
    let output = Command::new(git_same_binary())
        .args(["status", "--help"])
        .output()
        .expect("Failed to execute git-same");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("status"));
    assert!(stdout.contains("--uncommitted"));
    assert!(stdout.contains("--behind"));
}

#[test]
fn test_init_help() {
    let output = Command::new(git_same_binary())
        .args(["init", "--help"])
        .output()
        .expect("Failed to execute git-same");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Initialize"));
    assert!(stdout.contains("--force"));
}

#[test]
fn test_global_verbose_flag() {
    let output = Command::new(git_same_binary())
        .args(["-v", "--help"])
        .output()
        .expect("Failed to execute git-same");

    assert!(output.status.success());
}

#[test]
fn test_global_quiet_flag() {
    let output = Command::new(git_same_binary())
        .args(["-q", "--help"])
        .output()
        .expect("Failed to execute git-same");

    assert!(output.status.success());
}

#[test]
fn test_init_creates_config() {
    use tempfile::TempDir;

    let temp = TempDir::new().expect("Failed to create temp dir");
    let config_path = temp.path().join("gisa.config.toml");

    let output = Command::new(git_same_binary())
        .args(["init", "--path", config_path.to_str().unwrap()])
        .output()
        .expect("Failed to execute git-same");

    assert!(output.status.success(), "Init failed: {:?}", output);
    assert!(config_path.exists(), "Config file not created");

    // Verify content is valid TOML
    let content = std::fs::read_to_string(&config_path).expect("Failed to read config");
    assert!(content.contains("base_path"));
    assert!(content.contains("concurrency"));
}

#[test]
fn test_init_force_overwrites() {
    use tempfile::TempDir;

    let temp = TempDir::new().expect("Failed to create temp dir");
    let config_path = temp.path().join("gisa.config.toml");

    // Create initial file
    std::fs::write(&config_path, "# existing").expect("Failed to write");

    // Init without force should fail
    let output = Command::new(git_same_binary())
        .args(["init", "--path", config_path.to_str().unwrap()])
        .output()
        .expect("Failed to execute git-same");

    assert!(
        !output.status.success(),
        "Init without force should fail when file exists"
    );

    // Init with force should succeed
    let output = Command::new(git_same_binary())
        .args(["init", "--path", config_path.to_str().unwrap(), "--force"])
        .output()
        .expect("Failed to execute git-same");

    assert!(output.status.success(), "Init with force should succeed");

    // Verify content was overwritten
    let content = std::fs::read_to_string(&config_path).expect("Failed to read config");
    assert!(
        content.contains("base_path"),
        "Config should contain base_path"
    );
}

#[test]
fn test_status_nonexistent_workspace() {
    use tempfile::TempDir;

    let temp = TempDir::new().expect("Failed to create temp dir");
    let config_path = temp.path().join("config.toml");

    // Create a valid config so the test reaches workspace resolution
    Command::new(git_same_binary())
        .args(["init", "--path", config_path.to_str().unwrap()])
        .output()
        .expect("Failed to run init");

    let output = Command::new(git_same_binary())
        .args([
            "-C",
            config_path.to_str().unwrap(),
            "status",
            "--workspace",
            "nonexistent-workspace",
        ])
        .output()
        .expect("Failed to execute git-same");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("not found")
            || stderr.contains("No workspaces")
            || stderr.contains("No workspace configured"),
        "Expected workspace not found error, got: {}",
        stderr
    );
}

#[test]
fn test_sync_help() {
    let output = Command::new(git_same_binary())
        .args(["sync", "--help"])
        .output()
        .expect("Failed to execute git-same");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Sync"));
    assert!(stdout.contains("--workspace"));
    assert!(stdout.contains("--pull"));
    assert!(stdout.contains("--dry-run"));
}

#[test]
fn test_setup_help() {
    let output = Command::new(git_same_binary())
        .args(["setup", "--help"])
        .output()
        .expect("Failed to execute git-same");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("setup") || stdout.contains("Setup") || stdout.contains("wizard"));
}

#[test]
fn test_workspace_help() {
    let output = Command::new(git_same_binary())
        .args(["workspace", "--help"])
        .output()
        .expect("Failed to execute git-same");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("list"));
    assert!(stdout.contains("default"));
}

#[test]
fn test_workspace_list() {
    use tempfile::TempDir;

    let temp = TempDir::new().expect("Failed to create temp dir");
    let config_path = temp.path().join("config.toml");

    // Create a minimal valid config so the test doesn't depend on local config
    Command::new(git_same_binary())
        .args(["init", "--path", config_path.to_str().unwrap()])
        .output()
        .expect("Failed to run init");

    let output = Command::new(git_same_binary())
        .args(["-C", config_path.to_str().unwrap(), "workspace", "list"])
        .output()
        .expect("Failed to execute git-same");

    // Should succeed even with no workspaces
    assert!(output.status.success());
}

#[test]
fn test_missing_config_suggests_init() {
    let output = Command::new(git_same_binary())
        .args(["-C", "/tmp/nonexistent-gisa-config.toml", "sync"])
        .output()
        .expect("Failed to execute git-same");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("gisa init"),
        "Expected suggestion to run 'gisa init', got: {}",
        stderr
    );
}

#[test]
fn test_cli_subcommands_use_dashboard_subheadline() {
    use tempfile::TempDir;

    let temp = TempDir::new().expect("Failed to create temp dir");
    let home = temp.path().join("home");
    std::fs::create_dir_all(home.join(".config")).expect("Failed to create config dir");
    std::fs::create_dir_all(home.join(".cache")).expect("Failed to create cache dir");

    let config_path = temp.path().join("config.toml");
    let config_str = config_path
        .to_str()
        .expect("Config path is not valid UTF-8");

    let init_output = run_cli_with_env(&home, &["init", "--path", config_str, "--force"]);
    assert!(
        init_output.status.success(),
        "Init failed: {:?}",
        init_output
    );
    assert_banner_branding(&String::from_utf8_lossy(&init_output.stdout));

    let command_matrix: Vec<Vec<String>> = vec![
        vec![
            "-C".to_string(),
            config_str.to_string(),
            "sync".to_string(),
            "--dry-run".to_string(),
        ],
        vec![
            "-C".to_string(),
            config_str.to_string(),
            "status".to_string(),
        ],
        vec![
            "-C".to_string(),
            config_str.to_string(),
            "workspace".to_string(),
            "list".to_string(),
        ],
        vec![
            "-C".to_string(),
            config_str.to_string(),
            "workspace".to_string(),
            "default".to_string(),
        ],
        vec!["reset".to_string(), "--force".to_string()],
    ];

    for args in command_matrix {
        let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
        let output = run_cli_with_env(&home, &arg_refs);
        assert_banner_branding(&String::from_utf8_lossy(&output.stdout));
    }
}

#[test]
fn test_banner_source_no_legacy_version_subheadline() {
    let source = include_str!("../src/banner.rs");
    assert!(
        !source.contains("Mirror GitHub structure /orgs/repos/ to local file system  {}"),
        "Found legacy CLI subheadline format string in banner.rs"
    );
    assert!(
        !source.contains("local file system  Version"),
        "Found legacy versioned subheadline text in banner.rs"
    );
}
