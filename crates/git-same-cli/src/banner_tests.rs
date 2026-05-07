use super::*;

#[test]
fn subheadline_is_non_empty() {
    assert!(!subheadline().trim().is_empty());
}

#[test]
fn print_banner_executes_without_panicking() {
    print_banner();
}

#[test]
fn interpolate_stops_clamps_to_bounds() {
    let start = interpolate_stops(&[(0, 0, 0), (255, 255, 255)], -1.0);
    assert_eq!(start, (0, 0, 0));

    let end = interpolate_stops(&[(0, 0, 0), (255, 255, 255)], 2.0);
    assert_eq!(end, (255, 255, 255));
}

#[test]
fn cli_gradient_line_uses_truecolor_sequences() {
    let rendered = cli_gradient_line_with_force("AB", &GRADIENT_STOPS, true);
    assert!(rendered.contains("\u{1b}[38;2;59;130;246"));
    assert!(rendered.contains("\u{1b}[38;2;34;197;94"));
}

#[test]
fn cli_line5_styles_version_as_badge() {
    let version_display = format!("{:^6}", env!("CARGO_PKG_VERSION"));
    let rendered = cli_line5_with_force(&version_display, &GRADIENT_STOPS, true);
    assert!(rendered.contains("\u{1b}[48;2;"));
}

#[cfg(feature = "tui")]
#[test]
fn render_banner_handles_multiple_widths() {
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    for width in [64, 90] {
        let backend = TestBackend::new(width, 8);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                let area = frame.area();
                render_banner(frame, area);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let mut rendered = String::new();
        for y in 0..buffer.area.height {
            for x in 0..buffer.area.width {
                rendered.push_str(buffer[(x, y)].symbol());
            }
            rendered.push('\n');
        }

        assert!(rendered.contains(env!("CARGO_PKG_VERSION")));
    }
}
