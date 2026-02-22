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
    println!(
        "{}",
        style(format!(
            "              Mirror GitHub, locally.  {}\n",
            style(format!("v{}", env!("CARGO_PKG_VERSION"))).dim()
        ))
        .dim()
    );
}
