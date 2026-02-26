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
    std::fs::write(dot_dir.join("config.toml"), "[provider]\nkind = \"github\"\n").unwrap();

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
    std::fs::write(outer_dot.join("config.toml"), "[provider]\nkind = \"github\"\n").unwrap();

    // Inner workspace — should NOT appear because we stop recursing at outer
    let inner = outer.join("inner");
    let inner_dot = inner.join(".git-same");
    std::fs::create_dir_all(&inner_dot).unwrap();
    std::fs::write(inner_dot.join("config.toml"), "[provider]\nkind = \"github\"\n").unwrap();

    let found = scan_for_workspaces(temp.path(), 5);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0], std::fs::canonicalize(&outer).unwrap());
}
