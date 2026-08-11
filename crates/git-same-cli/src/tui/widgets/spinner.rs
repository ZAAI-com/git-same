//! Shared animated spinner frames for TUI screens.

const FRAMES: [char; 10] = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

/// Return the spinner frame for the current application tick.
pub(crate) fn frame(tick_count: u64) -> char {
    FRAMES[(tick_count % FRAMES.len() as u64) as usize]
}

#[cfg(test)]
#[path = "spinner_tests.rs"]
mod tests;
