use super::*;

#[test]
fn finish_glyph_warns_only_when_something_failed() {
    let clean = console::strip_ansi_codes(&finish_glyph(0).to_string()).to_string();
    let failed = console::strip_ansi_codes(&finish_glyph(2).to_string()).to_string();

    assert_eq!(clean, "✓");
    assert_eq!(failed, "⚠");
}
