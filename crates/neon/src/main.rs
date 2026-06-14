use anyhow::Result;
use clap::{Parser, Subcommand};

use neon_cli::doctor;
use neon_cli::repo::{self, InitArgs};

/// NeonOS CLI — developer environment diagnostics and tooling
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
    /// Repository setup and management commands
    Repo {
        #[command(subcommand)]
        command: RepoCommands,
    },
}

/// Subcommands for `neon repo`.
///
/// Future: `Harden` — apply GitHub settings and branch protection via the API.
#[derive(Subcommand)]
enum RepoCommands {
    /// Scaffold (or dry-run plan) a new repository.
    Init(InitArgs),
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Doctor => doctor::gather()?,
        Commands::Repo { command } => match command {
            RepoCommands::Init(args) => repo::init(args)?,
        },
    }

    Ok(())
}
