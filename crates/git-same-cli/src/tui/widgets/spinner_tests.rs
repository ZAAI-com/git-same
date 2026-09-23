use super::*;

#[test]
fn spinner_advances_through_braille_frames() {
    let rendered: String = (0..10).map(frame).collect();

    assert_eq!(rendered, "⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏");
}

#[test]
fn spinner_wraps_after_the_last_frame() {
    assert_eq!(frame(10), frame(0));
    assert_eq!(frame(11), frame(1));
}
