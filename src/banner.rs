//! ASCII banner for the gisa CLI.

use console::style;

const ART: &str = r"
 ██████╗ ██╗████████╗    ███████╗ █████╗ ███╗   ███╗███████╗
██╔════╝ ██║╚══██╔══╝    ██╔════╝██╔══██╗████╗ ████║██╔════╝
██║  ███╗██║   ██║       ███████╗███████║██╔████╔██║█████╗
██║   ██║██║   ██║       ╚════██║██╔══██║██║╚██╔╝██║██╔══╝
╚██████╔╝██║   ██║       ███████║██║  ██║██║ ╚═╝ ██║███████╗
 ╚═════╝ ╚═╝   ╚═╝       ╚══════╝╚═╝  ╚═╝╚═╝     ╚═╝╚══════╝";

/// Prints the gisa ASCII art banner to stdout.
pub fn print_banner() {
    println!("{}", style(ART).cyan().bold());
    let subtitle = format!(
        "Mirror GitHub structure /orgs/repos/ to local file system  {}",
        style(format!("Version {}", env!("CARGO_PKG_VERSION"))).dim()
    );
    // Center relative to the ASCII art width (~62 chars)
    let visible_len = format!(
        "Mirror GitHub structure /orgs/repos/ to local file system  Version {}",
        env!("CARGO_PKG_VERSION")
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
