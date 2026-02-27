use super::*;

#[test]
fn test_screens_exports_are_accessible() {
    let _ = &auth::render;
    let _ = &complete::render;
    let _ = &confirm::render;
    let _ = &orgs::render;
    let _ = &path::render;
    let _ = &provider::render;
    let _ = &requirements::render;
}
