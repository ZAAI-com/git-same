use super::*;
use std::path::PathBuf;

fn event_with_paths(paths: Vec<PathBuf>) -> Event {
    Event {
        paths,
        ..Default::default()
    }
}

#[test]
fn keeps_events_for_the_final_status_file() {
    let event = event_with_paths(vec![PathBuf::from("/ipc/status.json")]);
    assert!(event_touches_status_file(&event));
}

#[test]
fn skips_events_for_the_temp_file() {
    let event = event_with_paths(vec![PathBuf::from("/ipc/status.json.tmp")]);
    assert!(!event_touches_status_file(&event));
}

#[test]
fn keeps_events_when_any_path_is_the_status_file() {
    let event = event_with_paths(vec![
        PathBuf::from("/ipc/status.json.tmp"),
        PathBuf::from("/ipc/status.json"),
    ]);
    assert!(event_touches_status_file(&event));
}

#[test]
fn keeps_pathless_rescan_events() {
    let event = event_with_paths(Vec::new());
    assert!(event_touches_status_file(&event));
}
