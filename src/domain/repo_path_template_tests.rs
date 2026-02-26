use super::*;

#[test]
fn test_render_standard_template() {
    let template = RepoPathTemplate::new("{org}/{repo}");
    let path = template.render(Path::new("/tmp/base"), "github", "acme", "api");
    assert_eq!(path, PathBuf::from("/tmp/base/acme/api"));
}

#[test]
fn test_render_provider_template() {
    let template = RepoPathTemplate::new("{provider}/{org}/{repo}");
    let path = template.render(Path::new("/tmp/base"), "github", "acme", "api");
    assert_eq!(path, PathBuf::from("/tmp/base/github/acme/api"));
}

#[test]
fn test_scan_depth() {
    assert_eq!(RepoPathTemplate::new("{org}/{repo}").scan_depth(), 2);
    assert_eq!(
        RepoPathTemplate::new("{provider}/{org}/{repo}").scan_depth(),
        3
    );
    assert_eq!(RepoPathTemplate::new("code/{org}/{repo}").scan_depth(), 3);
}

#[test]
fn test_render_full_name() {
    let template = RepoPathTemplate::new("{org}/{repo}");
    let path = template.render_full_name(Path::new("/x"), "github", "acme/api");
    assert_eq!(path, Some(PathBuf::from("/x/acme/api")));
    assert!(template
        .render_full_name(Path::new("/x"), "github", "invalid")
        .is_none());
}

#[test]
fn test_render_sanitizes_path_components() {
    let template = RepoPathTemplate::new("{provider}/{org}/{repo}");
    let path = template.render(
        Path::new("/tmp/base"),
        "github/enterprise",
        "../acme",
        "api\\v2",
    );
    assert_eq!(
        path,
        PathBuf::from("/tmp/base/github_enterprise/___acme/api_v2")
    );
}
