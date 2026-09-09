use super::*;

fn make_bundle(root: &Path, name: &str, identifier: &str) -> PathBuf {
    let bundle = root.join(name);
    let helpers = bundle.join("Contents").join("Helpers");
    std::fs::create_dir_all(&helpers).unwrap();
    std::fs::create_dir_all(bundle.join("Contents").join("MacOS")).unwrap();
    std::fs::write(
        bundle.join("Contents").join("Info.plist"),
        format!(
            "<plist><dict><key>CFBundleIdentifier</key>\n  <string>{identifier}</string></dict></plist>"
        ),
    )
    .unwrap();
    write_executable(&helpers.join("git-same"), b"helper");
    // The main executable is what an app-owned agent actually runs.
    write_executable(
        &bundle.join("Contents").join("MacOS").join("git-same-app"),
        b"app",
    );
    std::fs::canonicalize(bundle).unwrap()
}

fn write_executable(path: &Path, bytes: &[u8]) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(path, bytes).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
}

fn record(kind: OwnerKind, owner: &Path, source: &Path) -> InstallRecord {
    InstallRecord {
        schema_version: InstallRecord::SCHEMA_VERSION,
        owner_kind: kind,
        owner_path: owner.to_path_buf(),
        source_binary: source.to_path_buf(),
        binary_version: "3.1.2".into(),
        binary_sha256: "x".into(),
        installed_at: String::new(),
        source_stamp: None,
    }
}

fn cli_source(path: &Path) -> HelperSource {
    HelperSource {
        owner_kind: OwnerKind::Cli,
        owner_path: path.to_path_buf(),
        source_binary: path.to_path_buf(),
        copy_from: path.to_path_buf(),
    }
}

#[test]
fn bundled_cli_and_app_binary_both_classify_as_the_app() {
    let dir = tempfile::tempdir().unwrap();
    let bundle = make_bundle(dir.path(), "Git Same & Co.app", APP_BUNDLE_ID);
    let helper = bundle.join("Contents/Helpers/git-same");
    let executable = bundle.join("Contents/MacOS/git-same-app");

    // Either entry point identifies the same owner, and either way the
    // installation runs the bundle's main executable in place: only that
    // file carries the bundle's TCC identity.
    for invoked in [helper, executable.clone()] {
        let source = classify(&invoked);
        assert_eq!(source.owner_kind, OwnerKind::App);
        assert_eq!(source.owner_path, bundle);
        assert_eq!(source.source_binary, executable);
        assert!(source.in_place());
        assert_eq!(source.program(Path::new("/managed/git-same")), executable);
    }
}

#[test]
fn a_cli_owner_runs_the_managed_copy_not_its_own_binary() {
    let source = classify(Path::new("/usr/local/bin/git-same"));
    assert_eq!(source.owner_kind, OwnerKind::Cli);
    assert!(!source.in_place());
    assert_eq!(
        source.program(Path::new("/managed/git-same")),
        Path::new("/managed/git-same")
    );
}

#[test]
fn the_expected_program_is_derived_from_the_owner_not_the_record() {
    // An agent installed by an older build recorded the copied helper as its
    // source; the expected program is still the bundle executable, so the
    // next repair re-renders the plist onto it.
    assert_eq!(
        program_for(
            OwnerKind::App,
            Path::new("/Applications/Git-Same.app"),
            Path::new("/managed/git-same")
        ),
        Path::new("/Applications/Git-Same.app/Contents/MacOS/git-same-app")
    );
    assert_eq!(
        program_for(
            OwnerKind::Cli,
            Path::new("/usr/local/bin/git-same"),
            Path::new("/managed/git-same")
        ),
        Path::new("/managed/git-same")
    );
}

#[test]
fn foreign_bundle_is_not_an_app_owner() {
    let dir = tempfile::tempdir().unwrap();
    let bundle = make_bundle(dir.path(), "Other.app", "com.example.other");
    let source = classify(&bundle.join("Contents/Helpers/git-same"));
    assert_eq!(source.owner_kind, OwnerKind::Cli);
}

#[test]
fn bare_contents_directory_is_not_a_bundle() {
    let source = classify(Path::new("/opt/tools/Contents/Helpers/git-same"));
    assert_eq!(source.owner_kind, OwnerKind::Cli);
    assert_eq!(
        source.owner_path,
        Path::new("/opt/tools/Contents/Helpers/git-same")
    );
}

#[cfg(unix)]
#[test]
fn formula_binary_maps_to_opt_under_a_custom_prefix() {
    let dir = tempfile::tempdir().unwrap();
    let prefix = std::fs::canonicalize(dir.path()).unwrap().join("my brew");
    let keg_bin = prefix.join("Cellar/git-same-cli/3.1.2/bin");
    std::fs::create_dir_all(&keg_bin).unwrap();
    std::fs::create_dir_all(prefix.join("opt")).unwrap();
    std::fs::write(keg_bin.join("git-same"), b"cli").unwrap();
    std::os::unix::fs::symlink(
        "../Cellar/git-same-cli/3.1.2",
        prefix.join("opt/git-same-cli"),
    )
    .unwrap();

    let real = keg_bin.join("git-same");
    let source = classify(&real);
    assert_eq!(source.owner_kind, OwnerKind::Cli);
    assert_eq!(
        source.source_binary,
        prefix.join("opt/git-same-cli/bin/git-same")
    );
    assert_eq!(source.copy_from, real);
}

#[test]
fn unverified_opt_path_falls_back_to_the_real_path() {
    let real = Path::new("/nonexistent/Cellar/git-same-cli/3.1.2/bin/git-same");
    assert_eq!(classify(real).source_binary, real);
}

#[test]
fn cask_source_copies_from_staging_but_records_the_final_app() {
    let source = cask_source(
        Path::new("/opt/homebrew/Caskroom/git-same/3.1.2/Git-Same.app/Contents/Helpers/git-same"),
        Path::new("/Users/ada/Applications/Git-Same.app"),
    );
    assert_eq!(source.owner_kind, OwnerKind::HomebrewCask);
    assert_eq!(
        source.source_binary,
        Path::new("/Users/ada/Applications/Git-Same.app/Contents/MacOS/git-same-app")
    );
    assert!(source.copy_from.starts_with("/opt/homebrew/Caskroom"));
    assert!(source.in_place());
}

#[test]
fn cargo_target_directories_are_dev_builds() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("worktree").join("target");
    std::fs::create_dir_all(target.join("debug")).unwrap();
    std::fs::write(target.join("CACHEDIR.TAG"), b"").unwrap();
    assert!(is_dev_build(&target.join("debug").join("git-same")));
    assert!(!is_dev_build(&dir.path().join(".cargo/bin/git-same")));
}

#[test]
fn nothing_installed_uses_the_caller() {
    let dir = tempfile::tempdir().unwrap();
    let binary = dir.path().join("git-same");
    write_executable(&binary, b"cli");
    let caller = cli_source(&binary);
    assert_eq!(
        select(None, false, Some(&caller), false, |_| false),
        Selection::Install(caller)
    );
}

#[test]
fn healthy_app_owned_helper_is_not_replaced_by_a_different_cli() {
    let existing = record(
        OwnerKind::App,
        Path::new("/Applications/Git-Same.app"),
        Path::new("/Applications/Git-Same.app/Contents/Helpers/git-same"),
    );
    let caller = cli_source(Path::new("/usr/local/bin/git-same"));
    assert_eq!(
        select(Some(&existing), true, Some(&caller), true, |_| false),
        Selection::Keep
    );
}

#[test]
fn app_caller_takes_over_from_a_standalone_owner() {
    let dir = tempfile::tempdir().unwrap();
    let bundle = make_bundle(dir.path(), "Git-Same.app", APP_BUNDLE_ID);
    let caller = classify(&bundle.join("Contents/Helpers/git-same"));
    let existing = record(
        OwnerKind::Cli,
        Path::new("/usr/local/bin/git-same"),
        Path::new("/usr/local/bin/git-same"),
    );
    assert_eq!(
        select(Some(&existing), true, Some(&caller), false, |_| false),
        Selection::Install(caller)
    );
}

#[test]
fn damaged_app_does_not_take_over_from_a_usable_recorded_cli() {
    let dir = tempfile::tempdir().unwrap();
    let bundle = make_bundle(dir.path(), "Git-Same.app", APP_BUNDLE_ID);
    let app_helper = bundle.join("Contents/Helpers/git-same");
    let caller = classify(&app_helper);
    // Damage the file the agent would actually run.
    std::fs::remove_file(bundle.join("Contents/MacOS/git-same-app")).unwrap();
    let recorded = dir.path().join("cli/git-same");
    write_executable(&recorded, b"usable cli");
    let existing = record(OwnerKind::Cli, &recorded, &recorded);

    assert_eq!(
        select(Some(&existing), true, Some(&caller), false, |_| false),
        Selection::Keep
    );
}

#[test]
fn owner_updates_from_its_own_changed_source_only() {
    let dir = tempfile::tempdir().unwrap();
    let owner_binary = dir.path().join("git-same");
    write_executable(&owner_binary, b"new");
    let existing = record(OwnerKind::Cli, &owner_binary, &owner_binary);
    let other = cli_source(Path::new("/somewhere/else/git-same"));

    match select(Some(&existing), true, Some(&other), false, |_| true) {
        Selection::Install(source) => assert_eq!(source.copy_from, owner_binary),
        other => panic!("expected update from the recorded owner, got {other:?}"),
    }
}

#[test]
fn missing_source_does_not_invalidate_an_intact_helper() {
    let existing = record(
        OwnerKind::Cli,
        Path::new("/gone/git-same"),
        Path::new("/gone/git-same"),
    );
    let caller = cli_source(Path::new("/usr/local/bin/git-same"));
    assert_eq!(
        select(Some(&existing), true, Some(&caller), false, |_| true),
        Selection::Keep
    );
}

#[test]
fn same_cask_bundle_keeps_cask_ownership_on_repair() {
    let dir = tempfile::tempdir().unwrap();
    let bundle = make_bundle(dir.path(), "Git-Same.app", APP_BUNDLE_ID);
    let caller = classify(&bundle.join("Contents/Helpers/git-same"));
    let mut existing = record(OwnerKind::HomebrewCask, &bundle, &caller.source_binary);
    existing.source_binary = dir.path().join("vanished");

    match select(Some(&existing), false, Some(&caller), false, |_| false) {
        Selection::Install(source) => assert_eq!(source.owner_kind, OwnerKind::HomebrewCask),
        other => panic!("unexpected {other:?}"),
    }
}

#[test]
fn automatic_recovery_never_installs_a_dev_build() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("target");
    std::fs::create_dir_all(target.join("debug")).unwrap();
    std::fs::write(target.join("CACHEDIR.TAG"), b"").unwrap();
    let caller = cli_source(&target.join("debug").join("git-same"));
    write_executable(&caller.copy_from, b"dev");

    assert!(matches!(
        select(None, false, Some(&caller), false, |_| false),
        Selection::Unavailable(_)
    ));
    assert_eq!(
        select(None, false, Some(&caller), true, |_| false),
        Selection::Install(caller)
    );
}

/// An explicit Start may record a cargo `target/` build as the source. From
/// then on, automatic recovery must not keep reinstalling it: `source_changed`
/// is true after every rebuild, so the monitor would restart on each one, and
/// the recorded path is trusted forever by launchd at login.
#[test]
fn automatic_recovery_never_reinstalls_a_recorded_dev_build() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("target");
    std::fs::create_dir_all(target.join("release")).unwrap();
    std::fs::write(target.join("CACHEDIR.TAG"), b"").unwrap();
    let binary = target.join("release").join("git-same");
    write_executable(&binary, b"dev build");
    let record = record(OwnerKind::Cli, &binary, &binary);

    // Helper present, source rebuilt.
    assert_eq!(
        select(Some(&record), true, None, false, |_| true),
        Selection::Keep
    );
    // Helper gone: repairing from a dev build is still refused.
    assert!(matches!(
        select(Some(&record), false, None, false, |_| true),
        Selection::Unavailable(_)
    ));

    // An explicit command may still do both.
    assert!(matches!(
        select(Some(&record), true, None, true, |_| true),
        Selection::Install(_)
    ));
    assert!(matches!(
        select(Some(&record), false, None, true, |_| true),
        Selection::Install(_)
    ));
}
