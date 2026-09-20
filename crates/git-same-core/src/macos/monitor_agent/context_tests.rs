use super::*;

const HOME: &str = "/Volumes/Data/people/Ada Lovelace";

fn check(uid: u32, env_home: Option<&str>, config_dir: bool, flag: bool) -> bool {
    check_overrides(
        uid,
        Path::new(HOME),
        env_home.map(Path::new),
        config_dir,
        flag,
    )
    .is_ok()
}

#[test]
fn real_user_environment_passes() {
    assert!(check(501, Some(HOME), false, false));
    assert!(check(501, None, false, false));
}

#[test]
fn root_is_rejected() {
    assert!(!check(0, Some(HOME), false, false));
}

#[test]
fn config_flag_is_rejected() {
    assert!(!check(501, Some(HOME), false, true));
}

#[test]
fn redirected_config_dir_is_rejected() {
    assert!(!check(501, Some(HOME), true, false));
}

#[test]
fn fake_home_is_rejected() {
    assert!(!check(501, Some("/tmp/fake-home"), false, false));
}

#[test]
fn paths_follow_the_home_directory_not_users() {
    let paths = MonitorAgentPaths::for_home(Path::new(HOME));
    assert_eq!(
        paths.helper,
        Path::new(HOME).join("Library/Application Support/com.zaai.git-same/monitor/git-same")
    );
    assert_eq!(
        paths.launch_agent,
        Path::new(HOME).join("Library/LaunchAgents/com.zaai.git-same.monitor.plist")
    );
    assert_eq!(
        paths.legacy_launch_agent,
        Path::new(HOME).join("Library/LaunchAgents/com.zaai.git-same.daemon.plist")
    );
    assert_eq!(paths.log_dir, Path::new(HOME).join("Library/Logs/git-same"));
    assert_eq!(
        paths.control_lock.parent(),
        Some(paths.managed_root.as_path())
    );
    assert!(paths.ipc.dir.ends_with(APP_GROUP_ID));
}

#[test]
fn domains_use_the_real_uid() {
    let user = UserContext {
        uid: 501,
        home: PathBuf::from(HOME),
    };
    assert_eq!(user.gui_domain(), "gui/501");
    assert_eq!(user.user_domain(), "user/501");
}
