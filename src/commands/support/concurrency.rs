use crate::operations::clone::MAX_CONCURRENCY;
use crate::output::Output;

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
mod tests {
    use super::*;
    use crate::output::{Output, Verbosity};

    fn quiet_output() -> Output {
        Output::new(Verbosity::Quiet, false)
    }

    #[test]
    fn test_concurrency_within_limit() {
        let output = quiet_output();
        assert_eq!(warn_if_concurrency_capped(4, &output), 4);
    }

    #[test]
    fn test_concurrency_at_limit() {
        let output = quiet_output();
        assert_eq!(
            warn_if_concurrency_capped(MAX_CONCURRENCY, &output),
            MAX_CONCURRENCY
        );
    }

    #[test]
    fn test_concurrency_above_limit() {
        let output = quiet_output();
        assert_eq!(
            warn_if_concurrency_capped(MAX_CONCURRENCY + 10, &output),
            MAX_CONCURRENCY
        );
    }
}
