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

fn default_config_path(home: &Path) -> PathBuf {
    home.join(".config").join("git-same").join("config.toml")
}

fn write_workspace_config(root: &Path) {
    let dot_dir = root.join(".git-same");
    std::fs::create_dir_all(&dot_dir).expect("Failed to create workspace metadata dir");
    std::fs::write(
        dot_dir.join("config.toml"),
        r#"[provider]
kind = "github"
"#,
    )
    .expect("Failed to write workspace config");
}

fn setup_registered_workspaces(home: &Path, roots: &[PathBuf]) {
    std::fs::create_dir_all(home.join(".config")).expect("Failed to create config dir");
    std::fs::create_dir_all(home.join(".cache")).expect("Failed to create cache dir");

    let init_output = run_cli_with_env(home, &["init", "--force"]);
    assert!(
        init_output.status.success(),
        "Init failed.\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&init_output.stdout),
        String::from_utf8_lossy(&init_output.stderr)
    );

    for root in roots {
        write_workspace_config(root);
    }

    let repos_root = home.join("repos");
    let repos_arg = repos_root
        .to_str()
        .expect("Repos root path is not valid UTF-8");
    let scan_output = run_cli_with_env(home, &["scan", repos_arg, "--register"]);
    assert!(
        scan_output.status.success(),
        "Scan/register failed.\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&scan_output.stdout),
        String::from_utf8_lossy(&scan_output.stderr)
    );
}

fn read_default_workspace(path: &Path) -> Option<String> {
    let content = std::fs::read_to_string(path).expect("Failed to read config file");
    let doc: toml::Value = toml::from_str(&content).expect("Failed to parse config TOML");
    doc.get("default_workspace")
        .and_then(|v| v.as_str())
        .map(ToString::to_string)
}

fn read_workspace_registry(path: &Path) -> Vec<String> {
    let content = std::fs::read_to_string(path).expect("Failed to read config file");
    let doc: toml::Value = toml::from_str(&content).expect("Failed to parse config TOML");
    doc.get("workspaces")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(ToString::to_string))
                .collect()
        })
        .unwrap_or_default()
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
    assert!(
        !stdout.contains("GT-SAME"),
        "Unexpected legacy GT-SAME banner text in stdout, got:\n{}",
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
        content.contains("concurrency"),
        "Config should contain concurrency setting"
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
            || stderr.contains("No workspace configured")
            || stderr.contains("No workspace config found")
            || stderr.contains("Configuration error"),
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
fn test_workspace_default_accepts_unique_folder_name() {
    use tempfile::TempDir;

    let temp = TempDir::new().expect("Failed to create temp dir");
    let home = temp.path().join("home");

    let ws_target = home.join("repos").join("team-a").join("work");
    let ws_other = home.join("repos").join("team-b").join("other");
    setup_registered_workspaces(&home, &[ws_target.clone(), ws_other]);

    let set_output = run_cli_with_env(&home, &["workspace", "default", "work"]);
    assert!(
        set_output.status.success(),
        "workspace default by folder name failed.\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&set_output.stdout),
        String::from_utf8_lossy(&set_output.stderr)
    );

    let config_path = default_config_path(&home);
    let default_workspace =
        read_default_workspace(&config_path).expect("Expected default_workspace to be set");
    assert!(
        default_workspace.ends_with("/repos/team-a/work"),
        "Expected default workspace to point at team-a/work, got '{}'",
        default_workspace
    );
}

#[test]
fn test_workspace_default_rejects_ambiguous_folder_name() {
    use tempfile::TempDir;

    let temp = TempDir::new().expect("Failed to create temp dir");
    let home = temp.path().join("home");

    let ws_a = home.join("repos").join("team-a").join("work");
    let ws_b = home.join("repos").join("team-b").join("work");
    setup_registered_workspaces(&home, &[ws_a, ws_b]);

    let output = run_cli_with_env(&home, &["workspace", "default", "work"]);
    assert!(
        !output.status.success(),
        "Expected ambiguous selector to fail.\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stderr = String::from_utf8_lossy(&output.stderr).to_lowercase();
    assert!(
        stderr.contains("ambiguous") && stderr.contains("explicit path"),
        "Expected ambiguous selector guidance in stderr, got:\n{}",
        stderr
    );

    let config_path = default_config_path(&home);
    let default_workspace = read_default_workspace(&config_path);
    assert!(
        default_workspace.is_none(),
        "default_workspace should remain unset on ambiguous selector, got {:?}",
        default_workspace
    );
}

#[test]
fn test_scan_register_requires_init() {
    use tempfile::TempDir;

    let temp = TempDir::new().expect("Failed to create temp dir");
    let home = temp.path().join("home");
    let repos = home.join("repos");
    let ws_root = repos.join("team").join("project");
    write_workspace_config(&ws_root);

    let repos_arg = repos.to_str().expect("Repos path is not valid UTF-8");
    let output = run_cli_with_env(&home, &["scan", repos_arg, "--register"]);
    assert!(
        !output.status.success(),
        "scan --register should fail when config is missing.\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Run 'gisa init' first") || stderr.contains("Config file not found"),
        "Expected init guidance in stderr, got:\n{}",
        stderr
    );

    assert!(
        !default_config_path(&home).exists(),
        "Config file should not be auto-created in this flow"
    );
}

#[test]
fn test_scan_register_uses_custom_config_path() {
    use tempfile::TempDir;

    let temp = TempDir::new().expect("Failed to create temp dir");
    let home = temp.path().join("home");
    let repos = home.join("repos");
    let ws_root = repos.join("team").join("project");
    write_workspace_config(&ws_root);

    let custom_config_path = temp.path().join("custom-config.toml");
    let custom_config_arg = custom_config_path
        .to_str()
        .expect("Custom config path is not valid UTF-8");
    let init_output = run_cli_with_env(&home, &["init", "--path", custom_config_arg, "--force"]);
    assert!(
        init_output.status.success(),
        "init --path failed.\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&init_output.stdout),
        String::from_utf8_lossy(&init_output.stderr)
    );

    let repos_arg = repos.to_str().expect("Repos path is not valid UTF-8");
    let scan_output = run_cli_with_env(
        &home,
        &["-C", custom_config_arg, "scan", repos_arg, "--register"],
    );
    assert!(
        scan_output.status.success(),
        "scan --register with custom config failed.\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&scan_output.stdout),
        String::from_utf8_lossy(&scan_output.stderr)
    );

    assert!(
        !default_config_path(&home).exists(),
        "Default config should not be required when -C is provided"
    );

    let workspaces = read_workspace_registry(&custom_config_path);
    assert_eq!(
        workspaces.len(),
        1,
        "Expected one registered workspace in custom config, got {:?}",
        workspaces
    );
    assert!(
        workspaces[0].ends_with("/repos/team/project"),
        "Expected registered workspace path to point at repos/team/project, got '{}'",
        workspaces[0]
    );
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
fn test_workspace_list_uses_canonical_banner() {
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

    let workspace_output = run_cli_with_env(&home, &["-C", config_str, "workspace", "list"]);
    assert!(workspace_output.status.success(), "workspace list failed");
    assert_banner_branding(&String::from_utf8_lossy(&workspace_output.stdout));
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
