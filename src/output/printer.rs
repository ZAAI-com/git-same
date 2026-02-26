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
}

impl Output {
    /// Creates a new output handler.
    pub fn new(verbosity: Verbosity, json: bool) -> Self {
        Self { verbosity, json }
    }

    /// Creates a quiet output handler.
    pub fn quiet() -> Self {
        Self::new(Verbosity::Quiet, false)
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

#[cfg(test)]
#[path = "printer_tests.rs"]
mod tests;
