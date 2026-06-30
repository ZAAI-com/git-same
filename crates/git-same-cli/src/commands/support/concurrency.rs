use git_same_core::operations::clone::MAX_CONCURRENCY;
use git_same_core::output::Output;

/// Warn if requested concurrency exceeds the maximum.
/// Returns the effective concurrency to use.
pub(crate) fn warn_if_concurrency_capped(requested: usize, output: &Output) -> usize {
    if requested > MAX_CONCURRENCY {
        output.warn(&format!(
            "Requested concurrency {} exceeds maximum {}. Using {} instead.",
            requested, MAX_CONCURRENCY, MAX_CONCURRENCY
        ));
        MAX_CONCURRENCY
    } else {
        requested
    }
}

#[cfg(test)]
#[path = "concurrency_tests.rs"]
mod tests;
