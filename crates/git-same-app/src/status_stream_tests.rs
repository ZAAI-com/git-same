use super::*;

fn ipc() -> IpcConfig {
    IpcConfig {
        dir: std::path::PathBuf::from("/group"),
    }
}

#[test]
fn status_file_carries_data_and_the_running_transition() {
    assert_eq!(
        relevance(Path::new("/group/status.json"), &ipc()),
        Relevance::DataAndMonitor
    );
}

#[test]
fn runtime_record_signals_start_and_exit() {
    assert_eq!(
        relevance(Path::new("/group/monitor-runtime.json"), &ipc()),
        Relevance::Monitor
    );
}

#[test]
fn temp_files_locks_and_caches_are_ignored() {
    for name in [
        ".status.json.123.0.tmp",
        ".monitor-runtime.json.123.1.tmp",
        "monitor.lock",
        "owner_types.json",
        "finder.sock",
        "preferences.json",
    ] {
        assert_eq!(
            relevance(&Path::new("/group").join(name), &ipc()),
            Relevance::Ignore,
            "{name}"
        );
    }
}
