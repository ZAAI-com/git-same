use super::*;

#[test]
fn scan_empty_directory_finds_nothing() {
    let temp = tempfile::tempdir().unwrap();
    let found = scan_for_workspaces(temp.path(), 3);
    assert!(found.is_empty());
}

#[test]
fn scan_finds_git_same_workspace() {
    let temp = tempfile::tempdir().unwrap();
    let ws_root = temp.path().join("my-workspace");
    let dot_dir = ws_root.join(".git-same");
    std::fs::create_dir_all(&dot_dir).unwrap();
    std::fs::write(
        dot_dir.join("config.toml"),
        "[provider]\nkind = \"github\"\n",
    )
    .unwrap();

    let found = scan_for_workspaces(temp.path(), 3);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0], std::fs::canonicalize(&ws_root).unwrap());
}

#[test]
fn scan_does_not_recurse_into_workspace() {
    let temp = tempfile::tempdir().unwrap();
    let outer = temp.path().join("outer");
    let outer_dot = outer.join(".git-same");
    std::fs::create_dir_all(&outer_dot).unwrap();
    std::fs::write(
        outer_dot.join("config.toml"),
        "[provider]\nkind = \"github\"\n",
    )
    .unwrap();

    // Inner workspace — should NOT appear because we stop recursing at outer
    let inner = outer.join("inner");
    let inner_dot = inner.join(".git-same");
    std::fs::create_dir_all(&inner_dot).unwrap();
    std::fs::write(
        inner_dot.join("config.toml"),
        "[provider]\nkind = \"github\"\n",
    )
    .unwrap();

    let found = scan_for_workspaces(temp.path(), 5);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0], std::fs::canonicalize(&outer).unwrap());
}

#[test]
fn run_register_with_custom_config_path_updates_registry() {
    let temp = tempfile::tempdir().unwrap();
    let scan_root = temp.path().join("scan-root");
    let ws_root = scan_root.join("team").join("project");
    let dot_dir = ws_root.join(".git-same");
    std::fs::create_dir_all(&dot_dir).unwrap();
    std::fs::write(
        dot_dir.join("config.toml"),
        "[provider]\nkind = \"github\"\n",
    )
    .unwrap();

    let custom_config_path = temp.path().join("custom-config.toml");
    std::fs::write(&custom_config_path, crate::config::Config::default_toml()).unwrap();

    let args = crate::cli::ScanArgs {
        path: Some(scan_root),
        depth: 5,
        register: true,
    };
    let output = crate::output::Output::quiet();
    run(&args, Some(&custom_config_path), &output).unwrap();

    let cfg = crate::config::Config::load_from(&custom_config_path).unwrap();
    assert_eq!(cfg.workspaces.len(), 1);
    let expected_suffix = std::path::Path::new("scan-root")
        .join("team")
        .join("project");
    assert!(
        std::path::Path::new(&cfg.workspaces[0]).ends_with(&expected_suffix),
        "Unexpected registered workspace path: {}",
        cfg.workspaces[0]
    );
}
