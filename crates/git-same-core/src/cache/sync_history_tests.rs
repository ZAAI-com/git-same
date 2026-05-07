use super::*;

fn sample_entry(index: usize) -> SyncHistoryEntry {
    SyncHistoryEntry {
        timestamp: format!("2026-01-{:02}T00:00:00Z", (index % 28) + 1),
        duration_secs: index as f64,
        success: index,
        failed: 0,
        skipped: 0,
        with_updates: index,
        cloned: index / 2,
        total_new_commits: index as u32,
    }
}

#[test]
fn load_missing_file_returns_empty_history() {
    let temp = tempfile::tempdir().unwrap();
    let manager = SyncHistoryManager {
        path: temp.path().join("sync-history.json"),
    };

    let entries = manager.load().unwrap();
    assert!(entries.is_empty());
}

#[test]
fn save_and_load_roundtrip_preserves_entries() {
    let temp = tempfile::tempdir().unwrap();
    let manager = SyncHistoryManager {
        path: temp.path().join("sync-history.json"),
    };
    let entries = vec![sample_entry(1), sample_entry(2), sample_entry(3)];

    manager.save(&entries).unwrap();
    let loaded = manager.load().unwrap();

    assert_eq!(loaded.len(), 3);
    assert_eq!(loaded[0].timestamp, entries[0].timestamp);
    assert_eq!(loaded[2].total_new_commits, entries[2].total_new_commits);
}

#[test]
fn save_caps_to_max_history_entries() {
    let temp = tempfile::tempdir().unwrap();
    let manager = SyncHistoryManager {
        path: temp.path().join("sync-history.json"),
    };

    let entries: Vec<_> = (0..75).map(sample_entry).collect();
    manager.save(&entries).unwrap();

    let loaded = manager.load().unwrap();
    assert_eq!(loaded.len(), 50);
    assert_eq!(loaded[0].duration_secs, 25.0);
    assert_eq!(loaded[49].duration_secs, 74.0);
}

#[test]
fn load_corrupt_json_returns_error() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("sync-history.json");
    std::fs::write(&path, "not-json").unwrap();

    let manager = SyncHistoryManager { path };
    let err = manager.load().unwrap_err();
    assert!(err.to_string().contains("Failed to parse sync history"));
}
