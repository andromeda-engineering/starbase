#[cfg(feature = "pantry")]
fn main() -> std::io::Result<()> {
    tui_pantry::run!(starbase_cli::doctor::widgets::pantry::ingredients())
}

#[cfg(not(feature = "pantry"))]
fn main() {
    eprintln!("Build with --features pantry to run the widget preview.");
    eprintln!("  cargo run --example widget_preview --features pantry");
    std::process::exit(1);
}
