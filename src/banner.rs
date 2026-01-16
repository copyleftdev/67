use std::io::{self, Write};
use tui_banner::{Banner, Gradient, Palette, Align, Fill};

/// Print a cinematic banner for 67
pub fn print_banner() {
    let banner = Banner::new("67")
        .unwrap()
        .gradient(Gradient::diagonal(Palette::from_hex(&[
            "#FF0080",
            "#FF00FF",
            "#8000FF",
            "#0080FF",
            "#00FFFF",
            "#00FF80",
            "#FFFF00",
            "#FF8000",
            "#FF0000",
        ])))
        .fill(Fill::Keep)
        .align(Align::Left)
        .padding(0);

    let output = banner.render();
    println!("{}", output);
    println!("  {} {}", 
        console::style("YouTube Downloader").white().bold(),
        console::style("• Fast • Concurrent • Focused").dim()
    );
    println!();
    
    let _ = io::stdout().flush();
}
