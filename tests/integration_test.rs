//! Integration tests for git-same CLI.
//!
//! These tests verify the CLI behavior as a whole.

use std::path::PathBuf;
use std::process::Command;

fn git_same_binary() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("target/debug/git-same");
    path
}

#[test]
fn test_help_command() {
    let output = Command::new(git_same_binary())
        .arg("--help")
        .output()
        .expect("Failed to execute git-same");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Mirror GitHub org/repo structure locally"));
    assert!(stdout.contains("clone"));
    assert!(stdout.contains("fetch"));
    assert!(stdout.contains("pull"));
    assert!(stdout.contains("status"));
    assert!(stdout.contains("init"));
    assert!(stdout.contains("completions"));
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
fn test_clone_help() {
    let output = Command::new(git_same_binary())
        .args(["clone", "--help"])
        .output()
        .expect("Failed to execute git-same");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Clone repositories"));
    assert!(stdout.contains("--dry-run"));
    assert!(stdout.contains("--concurrency"));
    assert!(stdout.contains("--org"));
}

#[test]
fn test_fetch_help() {
    let output = Command::new(git_same_binary())
        .args(["fetch", "--help"])
        .output()
        .expect("Failed to execute git-same");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Fetch updates"));
    assert!(stdout.contains("--dry-run"));
    assert!(stdout.contains("--no-skip-dirty"));
}

#[test]
fn test_pull_help() {
    let output = Command::new(git_same_binary())
        .args(["pull", "--help"])
        .output()
        .expect("Failed to execute git-same");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Pull updates"));
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
    assert!(stdout.contains("--dirty"));
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
fn test_completions_bash() {
    let output = Command::new(git_same_binary())
        .args(["completions", "bash"])
        .output()
        .expect("Failed to execute git-same");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("_gisa"));
    assert!(stdout.contains("complete -F"));
}

#[test]
fn test_completions_zsh() {
    let output = Command::new(git_same_binary())
        .args(["completions", "zsh"])
        .output()
        .expect("Failed to execute git-same");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("#compdef"));
}

#[test]
fn test_completions_fish() {
    let output = Command::new(git_same_binary())
        .args(["completions", "fish"])
        .output()
        .expect("Failed to execute git-same");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("complete"));
}

#[test]
fn test_clone_missing_argument() {
    let output = Command::new(git_same_binary())
        .arg("clone")
        .output()
        .expect("Failed to execute git-same");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("BASE_PATH") || stderr.contains("required"));
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
fn test_status_nonexistent_path() {
    let output = Command::new(git_same_binary())
        .args(["status", "/nonexistent/path/that/does/not/exist"])
        .output()
        .expect("Failed to execute git-same");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("does not exist") || stderr.contains("Path error"));
}

// Tests that require authentication are ignored by default
// Run with: cargo test -- --ignored

#[test]
#[ignore = "Requires GitHub authentication"]
fn test_clone_dry_run() {
    use tempfile::TempDir;

    let temp = TempDir::new().expect("Failed to create temp dir");

    let output = Command::new(git_same_binary())
        .args(["clone", temp.path().to_str().unwrap(), "--dry-run", "-v"])
        .output()
        .expect("Failed to execute git-same");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should show discovery progress or dry run output
    assert!(
        stdout.contains("repositories")
            || stdout.contains("Dry run")
            || stderr.contains("Authenticating"),
        "Expected discovery output, got stdout: {}, stderr: {}",
        stdout,
        stderr
    );
}
