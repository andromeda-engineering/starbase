use anyhow::Result;
use clap::{Parser, Subcommand};

use neon_cli::doctor;

/// NeonOS CLI – developer environment diagnostics and tooling
#[derive(Parser)]
#[command(name = "neon", version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Gather and display environment diagnostics
    Doctor,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Doctor => doctor::gather()?,
    }

    Ok(())
}
