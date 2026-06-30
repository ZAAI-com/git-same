use super::*;
use ratatui::backend::TestBackend;
use ratatui::Terminal;

fn render_output(repos: &[&OwnedRepo]) -> String {
    let backend = TestBackend::new(100, 12);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|frame| {
            let area = frame.area();
            render_owned_repos(frame, area, "Repositories", repos, 0);
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
fn repo_table_renders_title_headers_and_rows() {
    let public_repo = OwnedRepo::new("acme", git_same_core::types::Repo::test("rocket", "acme"));
    let mut private_repo = git_same_core::types::Repo::test("vault", "acme");
    private_repo.private = true;
    let private_repo = OwnedRepo::new("acme", private_repo);

    let rows = vec![&public_repo, &private_repo];
    let output = render_output(&rows);

    assert!(output.contains("Repositories"));
    assert!(output.contains("Name"));
    assert!(output.contains("Default Branch"));
    assert!(output.contains("Visibility"));
    assert!(output.contains("rocket"));
    assert!(output.contains("vault"));
    assert!(output.contains("public"));
    assert!(output.contains("private"));
}
