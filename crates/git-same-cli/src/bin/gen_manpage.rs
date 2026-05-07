//! Release-only helper: emit a roff(1) manpage for git-same on stdout.
//!
//! Build/run with `--features release-tools`. The default `cargo build` excludes
//! this binary because its `[[bin]]` entry sets `required-features`.

use clap::CommandFactory;
use clap_mangen::Man;
use git_same_cli::cli::Cli;
use std::{io, process};

fn main() {
    let cmd = Cli::command();
    let man = Man::new(cmd);
    if let Err(err) = man.render(&mut io::stdout()) {
        eprintln!("ERROR: failed to render manpage: {err}");
        process::exit(1);
    }
}
