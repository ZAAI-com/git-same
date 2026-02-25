//! ASCII banner for gisa — shared across CLI and TUI.

use console::style;

/// Banner lines 1-4 (shared between CLI and TUI).
const LINES: [&str; 4] = [
    " ██████╗ ██╗████████╗   ███████╗ █████╗ ███╗   ███╗███████╗",
    "██╔════╝ ██║╚══██╔══╝   ██╔════╝██╔══██╗████╗ ████║██╔════╝",
    "██║  ███╗██║   ██║█████╗███████╗███████║██╔████╔██║█████╗  ",
    "██║   ██║██║   ██║╚════╝╚════██║██╔══██║██║╚██╔╝██║██╔══╝  ",
];

/// Line 5 prefix (before version badge).
const LINE5_PREFIX: &str = "╚██████╔╝██║   ██║      ███████║██║  ██║██║ ╚═╝ ██║█";

/// Line 5 suffix (after version badge).
const LINE5_SUFFIX: &str = "╗";

/// Line 6.
const LAST_LINE: &str = " ╚═════╝ ╚═╝   ╚═╝      ╚══════╝╚═╝  ╚═╝╚═╝     ╚═╝╚══════╝";

/// Gradient color stops: Blue → Cyan → Green → Purple.
const GRADIENT_STOPS: [(u8, u8, u8); 4] = [
    (59, 130, 246), // Blue
    (6, 182, 212),  // Cyan
    (34, 197, 94),  // Green
    (147, 51, 234), // Purple
];

/// Prints the gisa ASCII art banner to stdout (CLI mode).
pub fn print_banner() {
    // Build full art from shared constants
    let version = env!("CARGO_PKG_VERSION");
    let version_display = format!("{:^6}", version);
    let line5 = format!("{LINE5_PREFIX}{version_display}{LINE5_SUFFIX}");
    let art = format!(
        "\n{}\n{}\n{}\n{}\n{}\n{}",
        LINES[0], LINES[1], LINES[2], LINES[3], line5, LAST_LINE
    );

    println!("{}", style(art).cyan().bold());
    let subtitle = format!(
        "Mirror GitHub structure /orgs/repos/ to local file system  {}",
        style(format!("Version {}", version)).dim()
    );
    let visible_len = format!(
        "Mirror GitHub structure /orgs/repos/ to local file system  Version {}",
        version
    )
    .len();
    let art_width = 62;
    let pad = if visible_len < art_width {
        (art_width - visible_len) / 2
    } else {
        0
    };
    println!("{}{}\n", " ".repeat(pad + 1), style(subtitle).dim());
}

// ---------------------------------------------------------------------------
// TUI rendering (feature-gated)
// ---------------------------------------------------------------------------

#[cfg(feature = "tui")]
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

/// Linearly interpolate between RGB color stops.
#[cfg(feature = "tui")]
pub(crate) fn interpolate_stops(stops: &[(u8, u8, u8)], t: f64) -> (u8, u8, u8) {
    let t = t.clamp(0.0, 1.0);
    let segments = stops.len() - 1;
    let scaled = t * segments as f64;
    let idx = (scaled as usize).min(segments - 1);
    let local_t = scaled - idx as f64;
    let (r1, g1, b1) = stops[idx];
    let (r2, g2, b2) = stops[idx + 1];
    let lerp = |a: u8, b: u8, t: f64| -> u8 { (a as f64 + (b as f64 - a as f64) * t) as u8 };
    (
        lerp(r1, r2, local_t),
        lerp(g1, g2, local_t),
        lerp(b1, b2, local_t),
    )
}

/// Apply a static gradient to a line of text.
#[cfg(feature = "tui")]
fn gradient_line<'a>(text: &'a str, stops: &[(u8, u8, u8)]) -> Line<'a> {
    let chars: Vec<&str> = text.split_inclusive(|_: char| true).collect();
    let len = chars.len().max(1);
    let spans: Vec<Span<'a>> = chars
        .into_iter()
        .enumerate()
        .map(|(i, ch)| {
            let t = i as f64 / (len - 1).max(1) as f64;
            let (r, g, b) = interpolate_stops(stops, t);
            Span::styled(
                ch.to_string(),
                Style::default()
                    .fg(Color::Rgb(r, g, b))
                    .add_modifier(Modifier::BOLD),
            )
        })
        .collect();
    Line::from(spans)
}

/// Compute the color for a character at normalized position `base_t`
/// during a left-to-right sweep animation at the given `phase`.
/// Returns the first stop color when the character is outside the wave.
#[cfg(feature = "tui")]
fn sweep_color(stops: &[(u8, u8, u8)], base_t: f64, phase: f64) -> (u8, u8, u8) {
    let wave_start = 2.0 * phase - 1.0;
    let wave_t = base_t - wave_start;
    if !(0.0..1.0).contains(&wave_t) {
        stops[0]
    } else {
        interpolate_stops(stops, wave_t)
    }
}

/// Apply an animated gradient sweep to a line of text (left-to-right wave).
/// `phase` in [0.0, 1.0] drives the sweep: 0.0 and 1.0 = all first-stop color,
/// 0.5 = full gradient visible.
#[cfg(feature = "tui")]
fn animated_gradient_line<'a>(text: &'a str, stops: &[(u8, u8, u8)], phase: f64) -> Line<'a> {
    let chars: Vec<&str> = text.split_inclusive(|_: char| true).collect();
    let len = chars.len().max(1);
    let spans: Vec<Span<'a>> = chars
        .into_iter()
        .enumerate()
        .map(|(i, ch)| {
            let base_t = i as f64 / (len - 1).max(1) as f64;
            let (r, g, b) = sweep_color(stops, base_t, phase);
            Span::styled(
                ch.to_string(),
                Style::default()
                    .fg(Color::Rgb(r, g, b))
                    .add_modifier(Modifier::BOLD),
            )
        })
        .collect();
    Line::from(spans)
}

/// Render the GIT-SAME banner with a static Blue → Cyan → Green → Purple gradient.
#[cfg(feature = "tui")]
pub fn render_banner(frame: &mut Frame, area: Rect) {
    let version = env!("CARGO_PKG_VERSION");
    let version_display = format!("{:^6}", version);
    let stops = &GRADIENT_STOPS;

    let mut banner_lines: Vec<Line> = Vec::new();
    for text in &LINES {
        banner_lines.push(gradient_line(text, stops));
    }

    // Line 5: gradient prefix + inverted version + gradient suffix
    let full_len =
        LINE5_PREFIX.chars().count() + version_display.len() + LINE5_SUFFIX.chars().count();
    let mut line5_spans: Vec<Span> = Vec::new();
    for (i, ch) in LINE5_PREFIX.split_inclusive(|_: char| true).enumerate() {
        let t = i as f64 / (full_len - 1).max(1) as f64;
        let (r, g, b) = interpolate_stops(stops, t);
        line5_spans.push(Span::styled(
            ch.to_string(),
            Style::default()
                .fg(Color::Rgb(r, g, b))
                .add_modifier(Modifier::BOLD),
        ));
    }
    let ver_pos = LINE5_PREFIX.chars().count();
    let ver_t = ver_pos as f64 / (full_len - 1).max(1) as f64;
    let (vr, vg, vb) = interpolate_stops(stops, ver_t);
    line5_spans.push(Span::styled(
        version_display,
        Style::default()
            .fg(Color::Black)
            .bg(Color::Rgb(vr, vg, vb))
            .add_modifier(Modifier::BOLD),
    ));
    let suffix_pos = ver_pos + 6;
    let t = suffix_pos as f64 / (full_len - 1).max(1) as f64;
    let (r, g, b) = interpolate_stops(stops, t);
    line5_spans.push(Span::styled(
        LINE5_SUFFIX.to_string(),
        Style::default()
            .fg(Color::Rgb(r, g, b))
            .add_modifier(Modifier::BOLD),
    ));
    banner_lines.push(Line::from(line5_spans));

    banner_lines.push(gradient_line(LAST_LINE, stops));

    let banner = Paragraph::new(banner_lines).centered();
    frame.render_widget(banner, area);
}

/// Render the GIT-SAME banner with animated gradient sweep (left-to-right wave).
/// `phase` in [0.0, 1.0] drives the sweep: 0.0 and 1.0 = all first-stop color,
/// 0.5 = full gradient visible.
#[cfg(feature = "tui")]
pub fn render_animated_banner(frame: &mut Frame, area: Rect, phase: f64) {
    let version = env!("CARGO_PKG_VERSION");
    let version_display = format!("{:^6}", version);
    let stops: &[(u8, u8, u8)] = &GRADIENT_STOPS;

    let mut banner_lines: Vec<Line> = Vec::new();
    for text in &LINES {
        banner_lines.push(animated_gradient_line(text, stops, phase));
    }

    // Line 5: sweep prefix + inverted version badge + sweep suffix
    let full_len =
        LINE5_PREFIX.chars().count() + version_display.len() + LINE5_SUFFIX.chars().count();
    let mut line5_spans: Vec<Span> = Vec::new();
    for (i, ch) in LINE5_PREFIX.split_inclusive(|_: char| true).enumerate() {
        let base_t = i as f64 / (full_len - 1).max(1) as f64;
        let (r, g, b) = sweep_color(stops, base_t, phase);
        line5_spans.push(Span::styled(
            ch.to_string(),
            Style::default()
                .fg(Color::Rgb(r, g, b))
                .add_modifier(Modifier::BOLD),
        ));
    }
    let ver_pos = LINE5_PREFIX.chars().count();
    let ver_base_t = ver_pos as f64 / (full_len - 1).max(1) as f64;
    let (vr, vg, vb) = sweep_color(stops, ver_base_t, phase);
    line5_spans.push(Span::styled(
        version_display,
        Style::default()
            .fg(Color::Black)
            .bg(Color::Rgb(vr, vg, vb))
            .add_modifier(Modifier::BOLD),
    ));
    let suffix_pos = ver_pos + 6;
    let suffix_base_t = suffix_pos as f64 / (full_len - 1).max(1) as f64;
    let (r, g, b) = sweep_color(stops, suffix_base_t, phase);
    line5_spans.push(Span::styled(
        LINE5_SUFFIX.to_string(),
        Style::default()
            .fg(Color::Rgb(r, g, b))
            .add_modifier(Modifier::BOLD),
    ));
    banner_lines.push(Line::from(line5_spans));

    banner_lines.push(animated_gradient_line(LAST_LINE, stops, phase));

    let banner = Paragraph::new(banner_lines).centered();
    frame.render_widget(banner, area);
}
