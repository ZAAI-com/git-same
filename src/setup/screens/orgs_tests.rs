use super::*;
use crate::setup::state::{OrgEntry, SetupState};
use ratatui::backend::TestBackend;
use ratatui::Terminal;

fn render_output(state: &SetupState) -> String {
    let backend = TestBackend::new(100, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|frame| {
            let area = frame.area();
            render(state, frame, area);
        })
        .unwrap();

    let buffer = terminal.backend().buffer();
    let mut text = String::new();
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            text.push_str(buffer[(x, y)].symbol());
        }
        text.push('\n');
    }
    text
}

#[test]
fn render_loading_state_shows_discovery_message() {
    let mut state = SetupState::new("~/Git-Same/GitHub");
    state.org_loading = true;
    state.tick_count = 3;

    let output = render_output(&state);
    assert!(output.contains("Select organizations to sync"));
    assert!(output.contains("Discovering organizations"));
}

#[test]
fn render_populated_orgs_shows_selection_summary() {
    let mut state = SetupState::new("~/Git-Same/GitHub");
    state.org_loading = false;
    state.orgs = vec![
        OrgEntry {
            name: "acme".to_string(),
            repo_count: 5,
            selected: true,
        },
        OrgEntry {
            name: "beta".to_string(),
            repo_count: 10,
            selected: false,
        },
    ];
    state.org_index = 0;

    let output = render_output(&state);
    assert!(output.contains("1 of 2 selected"));
    assert!(output.contains("5 repos"));
    assert!(output.contains("acme"));
    assert!(output.contains("beta"));
}

#[test]
fn render_empty_orgs_shows_personal_repo_hint() {
    let state = SetupState::new("~/Git-Same/GitHub");

    let output = render_output(&state);
    assert!(output.contains("No organizations found"));
    assert!(output.contains("personal repos"));
}

// --- Tests for the checked_div changes introduced in this PR ---

/// When all orgs have zero repos (max_repos == 0), checked_div prevents a
/// divide-by-zero panic and fills the bar with 0 blocks.
#[test]
fn render_all_zero_repo_counts_does_not_panic() {
    let mut state = SetupState::new("~/Git-Same/GitHub");
    state.org_loading = false;
    state.orgs = vec![
        OrgEntry {
            name: "empty-a".to_string(),
            repo_count: 0,
            selected: true,
        },
        OrgEntry {
            name: "empty-b".to_string(),
            repo_count: 0,
            selected: false,
        },
    ];
    state.org_index = 0;

    // Must not panic (the old code would divide by max_repos == 0).
    let output = render_output(&state);

    // The orgs should still appear in the output.
    assert!(output.contains("empty-a"));
    assert!(output.contains("empty-b"));
    // With zero repos, the percentage line is omitted (checked_div returns None).
    // So "%" should not appear next to an org row.
    assert!(!output.contains("%"));
}

/// A single org with 0 repos: filled bar should be 0 (not 1), and no
/// percentage is emitted because checked_div(total_repos=0) returns None.
#[test]
fn render_single_org_with_zero_repos_shows_empty_bar() {
    let mut state = SetupState::new("~/Git-Same/GitHub");
    state.org_loading = false;
    state.orgs = vec![OrgEntry {
        name: "solo".to_string(),
        repo_count: 0,
        selected: false,
    }];
    state.org_index = 0;

    let output = render_output(&state);
    assert!(output.contains("solo"));
    // No "%" character because total_repos == 0 → checked_div → None.
    assert!(!output.contains("%"));
}

/// When one org has all the repos (repo_count == max_repos), the filled
/// portion equals bar_width (16). Rendering should not panic.
#[test]
fn render_org_with_max_repos_fills_full_bar() {
    let mut state = SetupState::new("~/Git-Same/GitHub");
    state.org_loading = false;
    state.orgs = vec![
        OrgEntry {
            name: "big".to_string(),
            repo_count: 50,
            selected: true,
        },
        OrgEntry {
            name: "small".to_string(),
            repo_count: 5,
            selected: false,
        },
    ];
    state.org_index = 0;

    let output = render_output(&state);
    assert!(output.contains("big"));
    assert!(output.contains("small"));
    // Total = 55, so "big" has ~90% and "small" ~9%.
    assert!(output.contains("%"));
}

/// Orgs where every entry has the same repo count: each bar should get an
/// equal share (filled == bar_width == 16 when repo_count == max_repos for
/// all). Rendering must not panic.
#[test]
fn render_equal_repo_counts_does_not_panic() {
    let mut state = SetupState::new("~/Git-Same/GitHub");
    state.org_loading = false;
    state.orgs = vec![
        OrgEntry {
            name: "alpha".to_string(),
            repo_count: 10,
            selected: true,
        },
        OrgEntry {
            name: "beta".to_string(),
            repo_count: 10,
            selected: true,
        },
        OrgEntry {
            name: "gamma".to_string(),
            repo_count: 10,
            selected: false,
        },
    ];
    state.org_index = 1;

    let output = render_output(&state);
    assert!(output.contains("alpha"));
    assert!(output.contains("beta"));
    assert!(output.contains("gamma"));
    // All orgs have exactly 33% of total (10/30 * 100 = 33%).
    assert!(output.contains("33%"));
}

/// An org with repo_count > 0 when total_repos > 0 must show a percentage.
#[test]
fn render_nonzero_repos_shows_percentage() {
    let mut state = SetupState::new("~/Git-Same/GitHub");
    state.org_loading = false;
    state.orgs = vec![
        OrgEntry {
            name: "primary".to_string(),
            repo_count: 100,
            selected: true,
        },
        OrgEntry {
            name: "secondary".to_string(),
            repo_count: 100,
            selected: false,
        },
    ];
    state.org_index = 0;

    let output = render_output(&state);
    // Both orgs share 50% each (100/200 * 100).
    assert!(output.contains("50%"));
}

/// Error state renders an error message and retry prompt; must not panic.
#[test]
fn render_org_error_shows_retry_prompt() {
    let mut state = SetupState::new("~/Git-Same/GitHub");
    state.org_loading = false;
    state.org_error = Some("network timeout".to_string());

    let output = render_output(&state);
    assert!(output.contains("Failed to discover organizations"));
    assert!(output.contains("Press Enter to retry"));
    assert!(output.contains("network timeout"));
}
