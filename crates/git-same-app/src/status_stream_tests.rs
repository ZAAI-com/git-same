use super::*;

fn ipc() -> IpcConfig {
    IpcConfig {
        dir: std::path::PathBuf::from("/group"),
    }
}

fn targets() -> WatchTargets {
    WatchTargets::new(&ipc())
}

#[test]
fn status_file_carries_data_and_the_running_transition() {
    assert_eq!(
        targets().relevance(Path::new("/group/status.json")),
        Relevance::DataAndMonitor
    );
}

#[test]
fn runtime_record_signals_start_and_exit() {
    assert_eq!(
        targets().relevance(Path::new("/group/monitor-runtime.json")),
        Relevance::Monitor
    );
}

#[test]
fn temp_files_locks_and_caches_are_ignored() {
    let targets = targets();
    for name in [
        ".status.json.123.0.tmp",
        ".monitor-runtime.json.123.1.tmp",
        "monitor.lock",
        "owner_types.json",
        "finder.sock",
        "preferences.json",
    ] {
        assert_eq!(
            targets.relevance(&Path::new("/group").join(name)),
            Relevance::Ignore,
            "{name}"
        );
    }
}

#[test]
fn a_matching_name_in_another_directory_is_ignored() {
    let targets = targets();
    assert_eq!(
        targets.relevance(Path::new("/elsewhere/status.json")),
        Relevance::Ignore
    );
    assert_eq!(
        targets.relevance(Path::new("/group/nested/status.json")),
        Relevance::Ignore
    );
}

// Symlinked homes are a Unix concern, and `std::os::unix` does not exist
// on Windows.
#[cfg(unix)]
#[test]
fn a_symlinked_home_still_matches() {
    // FSEvents reports the resolved path while `IpcConfig.dir` keeps the
    // spelling derived from the passwd home. Byte-exact comparison dropped
    // every event on such a Mac.
    let temp = tempfile::TempDir::new().unwrap();
    let real = temp.path().join("real");
    std::fs::create_dir(&real).unwrap();
    let link = temp.path().join("link");
    std::os::unix::fs::symlink(&real, &link).unwrap();

    let targets = WatchTargets::new(&IpcConfig { dir: link.clone() });

    // The event names the resolved directory, the config names the link.
    assert_eq!(
        targets.relevance(&real.canonicalize().unwrap().join("status.json")),
        Relevance::DataAndMonitor
    );
    // The unresolved spelling keeps working.
    assert_eq!(
        targets.relevance(&link.join("monitor-runtime.json")),
        Relevance::Monitor
    );
}

#[test]
fn a_burst_collapses_into_one_update_on_the_trailing_edge() {
    let mut debouncer = Debouncer::default();
    let start = Instant::now();

    debouncer.record(Relevance::Monitor, start);
    // Still inside the window: nothing fires yet.
    assert_eq!(debouncer.take_due(start + Duration::from_millis(300)), None);

    // A second event pushes the deadline out instead of being dropped, which
    // the old leading-edge `if monitor_due.is_none()` guard did not do.
    debouncer.record(
        Relevance::DataAndMonitor,
        start + Duration::from_millis(300),
    );
    assert_eq!(debouncer.take_due(start + Duration::from_millis(500)), None);

    // One update carrying the most significant relevance of the whole burst.
    assert_eq!(
        debouncer.take_due(start + Duration::from_millis(701)),
        Some(Relevance::DataAndMonitor)
    );
    // And the state is reset, so a quiet timeout does not fire again.
    assert_eq!(debouncer.take_due(start + Duration::from_secs(10)), None);
}

#[test]
fn a_continuous_stream_still_updates_within_the_cap() {
    let mut debouncer = Debouncer::default();
    let start = Instant::now();

    // An event every 100ms would extend an uncapped trailing edge forever.
    let mut now = start;
    for _ in 0..40 {
        debouncer.record(Relevance::DataAndMonitor, now);
        now += Duration::from_millis(100);
    }

    assert!(
        debouncer
            .take_due(start + MAX_DEBOUNCE + Duration::from_millis(1))
            .is_some(),
        "the cap must release an update even while writes continue"
    );
}

#[test]
fn ignored_events_never_arm_the_debouncer() {
    let mut debouncer = Debouncer::default();
    let start = Instant::now();

    debouncer.record(Relevance::Ignore, start);

    assert_eq!(debouncer.timeout(start), Duration::from_secs(3600));
    assert_eq!(debouncer.take_due(start + Duration::from_secs(10)), None);
}

#[test]
fn rescan_and_pathless_events_count_as_new_data() {
    use notify::{event::Flag, EventKind};

    let targets = targets();

    // Path-bearing rescan: the path names the directory, not status.json.
    let rescan = Event::new(EventKind::Other)
        .set_flag(Flag::Rescan)
        .add_path(std::path::PathBuf::from("/group"));
    assert_eq!(
        event_relevance(&targets, &rescan),
        Relevance::DataAndMonitor
    );

    // Path-less rescan.
    let pathless_rescan = Event::new(EventKind::Other).set_flag(Flag::Rescan);
    assert_eq!(
        event_relevance(&targets, &pathless_rescan),
        Relevance::DataAndMonitor
    );

    // Path-less event without the rescan flag.
    let pathless = Event::new(EventKind::Other);
    assert_eq!(
        event_relevance(&targets, &pathless),
        Relevance::DataAndMonitor
    );

    // A normal irrelevant path is still ignored.
    let temp =
        Event::new(EventKind::Other).add_path(std::path::PathBuf::from("/group/monitor.lock"));
    assert_eq!(event_relevance(&targets, &temp), Relevance::Ignore);
}
