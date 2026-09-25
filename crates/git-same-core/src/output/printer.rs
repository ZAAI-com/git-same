use console::style;

/// Output verbosity level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Verbosity {
    /// No output except errors.
    Quiet = 0,
    /// Normal output.
    Normal = 1,
    /// Verbose output.
    Verbose = 2,
    /// Very verbose (debug) output.
    Debug = 3,
}

impl From<u8> for Verbosity {
    fn from(v: u8) -> Self {
        match v {
            0 => Verbosity::Quiet,
            1 => Verbosity::Normal,
            2 => Verbosity::Verbose,
            _ => Verbosity::Debug,
        }
    }
}

/// Output handler for consistent formatting.
#[derive(Debug, Clone)]
pub struct Output {
    verbosity: Verbosity,
    json: bool,
    quiet_requested: bool,
}

impl Output {
    /// Creates a new output handler.
    pub fn new(verbosity: Verbosity, json: bool) -> Self {
        Self {
            verbosity,
            json,
            quiet_requested: false,
        }
    }

    /// Creates a quiet output handler, as if quiet output was requested.
    pub fn quiet() -> Self {
        Self::new(Verbosity::Quiet, false).with_quiet_requested(true)
    }

    /// Records whether the user explicitly asked for quiet output (`-q`).
    ///
    /// The CLI's default level is also [`Verbosity::Quiet`], so the level
    /// alone cannot tell a plain run from `-q`. Only [`Output::summary`]
    /// looks at this.
    pub fn with_quiet_requested(mut self, quiet_requested: bool) -> Self {
        self.quiet_requested = quiet_requested;
        self
    }

    /// Prints a pre-formatted end-of-command summary line.
    ///
    /// Unlike [`Output::info`], this prints at the default level: only JSON
    /// mode or an explicit quiet request hides it.
    pub fn summary(&self, msg: &str) {
        if self.shows_summary() {
            println!("{}", msg);
        }
    }

    /// Prints an info message.
    pub fn info(&self, msg: &str) {
        if !self.json && self.verbosity >= Verbosity::Normal {
            println!("{} {}", style("→").cyan(), msg);
        }
    }

    /// Prints a success message.
    pub fn success(&self, msg: &str) {
        if !self.json && self.verbosity >= Verbosity::Normal {
            println!("{} {}", style("✓").green(), msg);
        }
    }

    /// Prints a warning message.
    pub fn warn(&self, msg: &str) {
        if !self.json && self.verbosity >= Verbosity::Normal {
            eprintln!("{} {}", style("⚠").yellow(), msg);
        }
    }

    /// Prints an error message.
    pub fn error(&self, msg: &str) {
        if !self.json {
            eprintln!("{} {}", style("✗").red(), msg);
        }
    }

    /// Prints a plain stdout line (no prefix).
    ///
    /// Useful for tabular/list output that should still respect quiet/json modes.
    pub fn plain(&self, msg: &str) {
        if !self.json && self.verbosity >= Verbosity::Normal {
            println!("{}", msg);
        }
    }

    /// Prints a verbose message.
    pub fn verbose(&self, msg: &str) {
        if !self.json && self.verbosity >= Verbosity::Verbose {
            println!("{} {}", style("·").dim(), msg);
        }
    }

    /// Prints a debug message.
    pub fn debug(&self, msg: &str) {
        if !self.json && self.verbosity >= Verbosity::Debug {
            println!("{} {}", style("⋅").dim(), style(msg).dim());
        }
    }

    /// Returns true if output is in JSON mode.
    pub fn is_json(&self) -> bool {
        self.json
    }

    /// Returns true if the user explicitly asked for quiet output.
    pub fn is_quiet_requested(&self) -> bool {
        self.quiet_requested
    }

    /// Returns true if [`Output::summary`] lines are printed.
    pub fn shows_summary(&self) -> bool {
        !self.json && !self.quiet_requested
    }

    /// Returns the current verbosity level.
    pub fn verbosity(&self) -> Verbosity {
        self.verbosity
    }
}

impl Default for Output {
    fn default() -> Self {
        Self::new(Verbosity::Normal, false)
    }
}

/// Format a count with appropriate styling.
pub fn format_count(count: usize, label: &str) -> String {
    format!("{} {}", style(count).cyan().bold(), label)
}

/// Format a success message.
pub fn format_success(msg: &str) -> String {
    format!("{} {}", style("✓").green(), msg)
}

/// Format an error message.
pub fn format_error(msg: &str) -> String {
    format!("{} {}", style("✗").red(), msg)
}

/// Format a warning message.
pub fn format_warning(msg: &str) -> String {
    format!("{} {}", style("⚠").yellow(), msg)
}

/// Format a skipped-item message (same glyph as the progress bars' skip lines).
pub fn format_skipped(msg: &str) -> String {
    format!("{} {}", style("→").dim(), msg)
}

#[cfg(test)]
#[path = "printer_tests.rs"]
mod tests;
