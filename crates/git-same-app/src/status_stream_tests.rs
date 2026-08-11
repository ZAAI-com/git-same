use super::*;
use notify::{event::Flag, EventKind};
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
fn keeps_path_bearing_rescan_events() {
    let event = Event::new(EventKind::Other)
        .set_flag(Flag::Rescan)
        .add_path(PathBuf::from("/ipc"));
    assert!(event_touches_status_file(&event));
}

#[test]
fn keeps_pathless_rescan_events() {
    let event = Event::new(EventKind::Other).set_flag(Flag::Rescan);
    assert!(event_touches_status_file(&event));
}

#[test]
fn keeps_pathless_events_without_rescan_flag() {
    let event = event_with_paths(Vec::new());
    assert!(event_touches_status_file(&event));
}
