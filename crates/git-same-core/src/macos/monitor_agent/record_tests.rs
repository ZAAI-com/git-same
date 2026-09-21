use super::*;

fn sample() -> InstallRecord {
    InstallRecord {
        schema_version: InstallRecord::SCHEMA_VERSION,
        owner_kind: OwnerKind::HomebrewCask,
        owner_path: PathBuf::from("/Applications/Git-Same.app"),
        source_binary: PathBuf::from("/Applications/Git-Same.app/Contents/Helpers/git-same"),
        binary_version: "3.1.2".to_string(),
        binary_sha256: "abc".to_string(),
        installed_at: "2026-09-20T00:00:00Z".to_string(),
        source_stamp: Some((10, 20)),
    }
}

#[test]
fn round_trips_and_uses_snake_case_owner_kinds() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("monitor").join("install.json");
    sample().save(&path).unwrap();
    assert!(std::fs::read_to_string(&path)
        .unwrap()
        .contains("\"homebrew_cask\""));
    assert_eq!(InstallRecord::load(&path).unwrap(), Some(sample()));
}

#[test]
fn missing_record_is_none() {
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(
        InstallRecord::load(&dir.path().join("install.json")).unwrap(),
        None
    );
}

#[test]
fn unsupported_schema_is_an_actionable_error_and_is_not_overwritten() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("install.json");
    let future = r#"{"schema_version": 2, "owner": "someone"}"#;
    std::fs::write(&path, future).unwrap();

    let error = InstallRecord::load(&path).unwrap_err().to_string();
    assert!(error.contains("unsupported schema version"));
    assert!(error.contains("--uninstall"));
    assert_eq!(std::fs::read_to_string(&path).unwrap(), future);
}

#[test]
fn corrupt_record_is_an_error() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("install.json");
    std::fs::write(&path, "{not json").unwrap();
    assert!(InstallRecord::load(&path).is_err());
}

#[test]
fn sha256_matches_a_known_digest() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("file");
    std::fs::write(&path, b"abc").unwrap();
    assert_eq!(
        sha256_file(&path).unwrap(),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}

#[test]
fn only_app_kinds_count_as_app_owned() {
    assert!(OwnerKind::HomebrewCask.is_app());
    assert!(OwnerKind::App.is_app());
    assert!(!OwnerKind::Cli.is_app());
}
