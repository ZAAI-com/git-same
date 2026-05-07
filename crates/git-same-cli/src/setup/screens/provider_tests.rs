use super::*;
use crate::setup::state::SetupState;
use git_same_core::types::ProviderKind;
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
fn provider_description_matches_expected_labels() {
    assert!(provider_description(ProviderKind::GitHub).contains("github.com"));
    assert!(provider_description(ProviderKind::GitHubEnterprise).contains("coming soon"));
    assert!(provider_description(ProviderKind::GitLab).contains("gitlab.com"));
    assert!(provider_description(ProviderKind::GitLabSelfManaged).contains("coming soon"));
    assert!(provider_description(ProviderKind::Codeberg).contains("codeberg.org"));
    assert!(provider_description(ProviderKind::Bitbucket).contains("bitbucket.org"));
}

#[test]
fn render_provider_screen_shows_options_and_selection() {
    let state = SetupState::new("~/Git-Same/GitHub");

    let output = render_output(&state);
    assert!(output.contains("Select your Git provider"));
    assert!(output.contains("GitHub"));
    assert!(output.contains("GitHub Enterprise (coming soon)"));
    assert!(output.contains("Codeberg.org (coming soon)"));
}
