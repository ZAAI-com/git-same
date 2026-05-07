//! Release-only helper: emit a shell completion script for git-same on stdout.
//!
//! Build/run with `--features release-tools`. The default `cargo build` excludes
//! this binary because its `[[bin]]` entry sets `required-features`.
//!
//! Usage: gen-completions <bash|zsh|fish|elvish|powershell>

use clap::CommandFactory;
use clap_complete::{generate, Shell};
use git_same_cli::cli::Cli;
use std::{env, io, process};

fn main() {
    let mut args = env::args().skip(1);
    let shell_arg = match args.next() {
        Some(s) => s,
        None => {
            eprintln!("Usage: gen-completions <bash|zsh|fish|elvish|powershell>");
            process::exit(2);
        }
    };

    let shell: Shell = match shell_arg.parse() {
        Ok(s) => s,
        Err(_) => {
            eprintln!("ERROR: unknown shell '{shell_arg}'");
            process::exit(2);
        }
    };

    let mut cmd = Cli::command();
    generate(shell, &mut cmd, "git-same", &mut io::stdout());
}
