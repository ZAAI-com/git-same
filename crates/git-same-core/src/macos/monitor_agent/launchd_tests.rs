use super::*;

const PRINT_RUNNING: &str = "gui/501/com.zaai.git-same.monitor = {\n\tactive count = 1\n\tpath = /Users/ada/Library/LaunchAgents/com.zaai.git-same.monitor.plist\n\tstate = running\n\n\tprogram = /Users/ada/Library/Application Support/com.zaai.git-same/monitor/git-same\n\targuments = {\n\t\t/x/git-same\n\t\tmonitor\n\t}\n\n\tpid = 4242\n\tendpoints = {\n\t\tpid = 1\n\t}\n}\n";

const PRINT_IDLE: &str = "gui/501/com.zaai.git-same.monitor = {\n\tstate = not running\n\tprogram = /x/git-same\n\tlast exit code = 0\n}\n";

#[test]
fn parses_pid_and_program_from_top_level_keys_only() {
    let info = parse_service(PRINT_RUNNING);
    assert!(info.loaded);
    assert_eq!(info.pid, Some(4242));
    assert_eq!(
        info.program.as_deref(),
        Some("/Users/ada/Library/Application Support/com.zaai.git-same/monitor/git-same")
    );
}

#[test]
fn loaded_but_idle_service_has_no_pid() {
    let info = parse_service(PRINT_IDLE);
    assert!(info.loaded);
    assert_eq!(info.pid, None);
}

#[test]
fn parses_both_disabled_formats() {
    let current = "disabled services = {\n\t\"com.other\" => enabled\n\t\"com.zaai.git-same.monitor\" => disabled\n}\n";
    let older = "disabled services = {\n\t\"com.zaai.git-same.monitor\" => true\n}\n";
    let enabled = "disabled services = {\n\t\"com.zaai.git-same.monitor\" => enabled\n\t\"com.zaai.git-same.monitor.other\" => disabled\n}\n";
    assert!(parse_disabled(current, "com.zaai.git-same.monitor"));
    assert!(parse_disabled(older, "com.zaai.git-same.monitor"));
    assert!(!parse_disabled(enabled, "com.zaai.git-same.monitor"));
    assert!(!parse_disabled("", "com.zaai.git-same.monitor"));
}

#[test]
fn recognizes_service_not_found() {
    let by_code = CommandOutput {
        code: Some(113),
        ..Default::default()
    };
    let by_text = CommandOutput {
        code: Some(1),
        stderr: "Boot-out failed: 3: No such process".into(),
        ..Default::default()
    };
    let real_failure = CommandOutput {
        code: Some(5),
        stderr: "Input/output error".into(),
        ..Default::default()
    };
    assert!(is_not_found(&by_code));
    assert!(is_not_found(&by_text));
    assert!(!is_not_found(&real_failure));
}
