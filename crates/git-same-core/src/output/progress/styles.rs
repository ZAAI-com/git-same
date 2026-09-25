use console::{style, StyledObject};
use indicatif::ProgressStyle;

/// Default spinner style frames.
const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

/// Creates a default spinner style.
pub fn spinner_style() -> ProgressStyle {
    ProgressStyle::default_spinner()
        .tick_strings(SPINNER_FRAMES)
        .template("{spinner:.cyan} {msg}")
        .expect("Invalid spinner template")
}

/// Creates a progress bar style.
pub fn progress_style() -> ProgressStyle {
    ProgressStyle::default_bar()
        .template("{spinner:.cyan} [{bar:40.cyan/dim}] {pos}/{len} {msg}")
        .expect("Invalid progress template")
        .progress_chars("━╸─")
}

/// Glyph for a finished phase bar: a check, or a warning sign if anything failed.
pub fn finish_glyph(failed: usize) -> StyledObject<&'static str> {
    if failed > 0 {
        style("⚠").yellow()
    } else {
        style("✓").green()
    }
}

#[cfg(test)]
#[path = "styles_tests.rs"]
mod tests;
