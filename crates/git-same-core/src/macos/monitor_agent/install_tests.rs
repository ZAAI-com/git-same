use super::*;
use crate::macos::monitor_agent::fake::{FakeCodesign, FakeSystem};
use crate::macos::monitor_agent::record::OwnerKind;

struct Env {
    _dir: tempfile::TempDir,
    paths: MonitorAgentPaths,
    system: FakeSystem,
    source: HelperSource,
}

fn env() -> Env {
    let dir = tempfile::tempdir().unwrap();
    let paths = MonitorAgentPaths::for_home(&dir.path().join("home"));
    let binary = dir.path().join("dist").join("git-same");
    write_executable(&binary, b"helper v1");
    let source = HelperSource {
        owner_kind: OwnerKind::Cli,
        owner_path: binary.clone(),
        source_binary: binary.clone(),
        copy_from: binary,
    };
    Env {
        _dir: dir,
        paths,
        system: FakeSystem::new(),
        source,
    }
}

fn write_executable(path: &Path, bytes: &[u8]) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, bytes).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
}

fn install(env: &Env, bytes: &[u8], plist: &str) {
    write_executable(&env.source.copy_from, bytes);
    let installer = Installer::new(&env.system, &env.paths);
    let staged = installer.stage(env.source.clone()).unwrap();
    installer.activate(&staged, plist).unwrap();
    installer.commit(&staged, "3.1.2").unwrap();
}

fn leftovers(env: &Env) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(&env.paths.managed_root)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

#[test]
fn fresh_install_writes_helper_plist_and_record() {
    let env = env();
    install(&env, b"helper v1", "<plist/>");

    assert_eq!(std::fs::read(&env.paths.helper).unwrap(), b"helper v1");
    assert!(is_executable(&env.paths.helper));
    assert_eq!(
        std::fs::read_to_string(&env.paths.launch_agent).unwrap(),
        "<plist/>"
    );
    let record = InstallRecord::load(&env.paths.install_record)
        .unwrap()
        .unwrap();
    assert_eq!(
        record.binary_sha256,
        sha256_file(&env.paths.helper).unwrap()
    );
    assert_eq!(record.binary_version, "3.1.2");
    assert_eq!(leftovers(&env), vec!["git-same", "install.json"]);
}

#[cfg(unix)]
#[test]
fn plist_is_not_group_or_world_writable() {
    use std::os::unix::fs::PermissionsExt;
    let env = env();
    install(&env, b"helper v1", "<plist/>");
    let mode = std::fs::metadata(&env.paths.launch_agent)
        .unwrap()
        .permissions()
        .mode();
    assert_eq!(mode & 0o022, 0);
}

#[test]
fn missing_source_changes_nothing() {
    let env = env();
    std::fs::remove_file(&env.source.copy_from).unwrap();
    let error = Installer::new(&env.system, &env.paths)
        .stage(env.source.clone())
        .unwrap_err();
    assert!(matches!(error, MonitorAgentError::MissingSource(_)));
    assert!(!env.paths.helper.exists());
}

#[cfg(unix)]
#[test]
fn non_executable_source_is_rejected() {
    use std::os::unix::fs::PermissionsExt;
    let env = env();
    std::fs::set_permissions(
        &env.source.copy_from,
        std::fs::Permissions::from_mode(0o644),
    )
    .unwrap();
    let error = Installer::new(&env.system, &env.paths)
        .stage(env.source.clone())
        .unwrap_err();
    assert!(matches!(error, MonitorAgentError::InvalidExecutable { .. }));
}

#[test]
fn copy_failure_leaves_the_working_installation_alone() {
    let env = env();
    install(&env, b"helper v1", "<plist v1/>");
    write_executable(&env.source.copy_from, b"helper v2");
    env.system
        .with(|s| s.fail_copy_to = Some("git-same.new".to_string()));

    assert!(Installer::new(&env.system, &env.paths)
        .stage(env.source.clone())
        .is_err());
    assert_eq!(std::fs::read(&env.paths.helper).unwrap(), b"helper v1");
    assert_eq!(leftovers(&env), vec!["git-same", "install.json"]);
}

#[test]
fn copy_that_does_not_match_its_source_is_rejected() {
    let env = env();
    env.system.with(|s| s.corrupt_copies = true);
    let error = Installer::new(&env.system, &env.paths)
        .stage(env.source.clone())
        .unwrap_err();
    assert!(matches!(error, MonitorAgentError::InvalidExecutable { .. }));
    assert!(!env.paths.managed_root.join("git-same.new").exists());
}

#[test]
fn rollback_restores_the_previous_installation() {
    let env = env();
    install(&env, b"helper v1", "<plist v1/>");
    let record_before = std::fs::read(&env.paths.install_record).unwrap();

    write_executable(&env.source.copy_from, b"helper v2");
    let installer = Installer::new(&env.system, &env.paths);
    let staged = installer.stage(env.source.clone()).unwrap();
    installer.activate(&staged, "<plist v2/>").unwrap();
    assert_eq!(std::fs::read(&env.paths.helper).unwrap(), b"helper v2");

    installer.rollback().unwrap();

    assert_eq!(std::fs::read(&env.paths.helper).unwrap(), b"helper v1");
    assert_eq!(
        std::fs::read_to_string(&env.paths.launch_agent).unwrap(),
        "<plist v1/>"
    );
    assert_eq!(
        std::fs::read(&env.paths.install_record).unwrap(),
        record_before
    );
    assert_eq!(leftovers(&env), vec!["git-same", "install.json"]);
}

#[test]
fn rollback_of_a_first_install_removes_the_new_files() {
    let env = env();
    let installer = Installer::new(&env.system, &env.paths);
    let staged = installer.stage(env.source.clone()).unwrap();
    installer.activate(&staged, "<plist/>").unwrap();

    installer.rollback().unwrap();

    assert!(!env.paths.helper.exists());
    assert!(!env.paths.launch_agent.exists());
    assert!(!env.paths.install_record.exists());
}

#[test]
fn interrupted_replacement_is_recovered_by_the_next_operation() {
    let env = env();
    install(&env, b"helper v1", "<plist v1/>");
    write_executable(&env.source.copy_from, b"helper v2");
    {
        // Crash right after the live files were replaced: no commit.
        let installer = Installer::new(&env.system, &env.paths);
        let staged = installer.stage(env.source.clone()).unwrap();
        installer.activate(&staged, "<plist v2/>").unwrap();
    }
    assert!(env.paths.transaction_record.exists());

    let recovered = Installer::new(&env.system, &env.paths)
        .recover_interrupted()
        .unwrap();

    assert!(recovered);
    assert_eq!(std::fs::read(&env.paths.helper).unwrap(), b"helper v1");
    assert_eq!(
        std::fs::read_to_string(&env.paths.launch_agent).unwrap(),
        "<plist v1/>"
    );
    assert!(!env.paths.transaction_record.exists());
}

#[test]
fn crash_before_activation_only_leaves_a_staged_file_to_sweep() {
    let env = env();
    install(&env, b"helper v1", "<plist v1/>");
    write_executable(&env.source.copy_from, b"helper v2");
    let _ = Installer::new(&env.system, &env.paths)
        .stage(env.source.clone())
        .unwrap();

    let recovered = Installer::new(&env.system, &env.paths)
        .recover_interrupted()
        .unwrap();

    assert!(!recovered);
    assert_eq!(std::fs::read(&env.paths.helper).unwrap(), b"helper v1");
    assert_eq!(leftovers(&env), vec!["git-same", "install.json"]);
}

#[test]
fn remove_installation_keeps_locks_logs_and_unrelated_files() {
    let env = env();
    install(&env, b"helper v1", "<plist/>");
    std::fs::write(&env.paths.control_lock, b"").unwrap();
    std::fs::create_dir_all(&env.paths.log_dir).unwrap();
    std::fs::write(&env.paths.stderr_log, b"log").unwrap();
    let unrelated = env
        .paths
        .launch_agent
        .with_file_name("com.example.other.plist");
    std::fs::write(&unrelated, b"x").unwrap();

    Installer::new(&env.system, &env.paths)
        .remove_installation()
        .unwrap();

    assert!(!env.paths.helper.exists());
    assert!(!env.paths.launch_agent.exists());
    assert!(!env.paths.install_record.exists());
    assert!(env.paths.control_lock.exists());
    assert!(env.paths.stderr_log.exists());
    assert!(unrelated.exists());
}

// ------------------------------------------------- signature verification
//
// Without a scripted `codesign` every test binary is unsigned, so
// `verify_signature` returns early and none of these three checks runs.

#[test]
fn a_signed_source_whose_copy_keeps_its_identity_is_accepted() {
    let env = env();
    env.system
        .with(|s| s.codesign = Some(FakeCodesign::signed("57KL6Y7V32")));

    Installer::new(&env.system, &env.paths)
        .stage(env.source.clone())
        .unwrap();
}

#[test]
fn a_copy_that_lost_the_app_group_entitlement_is_rejected() {
    let env = env();
    env.system.with(|s| {
        s.codesign = Some(FakeCodesign {
            team: "57KL6Y7V32".to_string(),
            // Only the source carries it; the staged copy does not.
            app_group: vec![env.source.copy_from.display().to_string()],
            ..FakeCodesign::default()
        })
    });

    let err = Installer::new(&env.system, &env.paths)
        .stage(env.source.clone())
        .unwrap_err();

    assert!(
        err.to_string().contains("app-group entitlement"),
        "got: {err}"
    );
    assert_eq!(leftovers(&env), Vec::<String>::new(), "staged copy swept");
}

/// A `codesign` that cannot read entitlements used to report "no app group",
/// which silently skipped the check and shipped a helper that cannot write
/// the group container.
#[test]
fn an_unreadable_entitlement_blob_is_rejected_rather_than_assumed_empty() {
    let env = env();
    env.system.with(|s| {
        s.codesign = Some(FakeCodesign {
            team: "57KL6Y7V32".to_string(),
            entitlements_fail: vec![String::new()],
            ..FakeCodesign::default()
        })
    });

    let err = Installer::new(&env.system, &env.paths)
        .stage(env.source.clone())
        .unwrap_err();

    assert!(err.to_string().contains("codesign"), "got: {err}");
}

#[test]
fn a_copy_whose_signature_does_not_verify_is_rejected() {
    let env = env();
    env.system.with(|s| {
        s.codesign = Some(FakeCodesign {
            team: "57KL6Y7V32".to_string(),
            verify_fails: vec![String::new()],
            ..FakeCodesign::default()
        })
    });

    let err = Installer::new(&env.system, &env.paths)
        .stage(env.source.clone())
        .unwrap_err();

    assert!(err.to_string().contains("code signature"), "got: {err}");
}
