use anyhow::Result;
use clap::{Parser, Subcommand};

use neon_cli::doctor;
use neon_cli::install::{self, InstallAppsArgs};
use neon_cli::repo::{self, InitArgs};
use neon_cli::setup::{
    self, DiagnosticsArgs, DockerLoginArgs, DockerLogoutArgs, DockerShowArgs, GitIdentityArgs,
    InstallPackagesArgs, NpmTokenArgs, PickShellArgs, PickTerminalArgs, SetupClaudeArgs,
};

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
    /// Machine setup and environment configuration
    Setup {
        #[command(subcommand)]
        command: SetupCommands,
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

/// Subcommands for `neon setup`.
#[derive(Subcommand)]
enum SetupCommands {
    /// Probe and report machine capabilities (OS, shells, tools)
    Detect,
    /// Bootstrap the Claude/agent environment (junctions, skills, global config)
    Claude(SetupClaudeArgs),
    /// Set git user.name / user.email for local or global scope
    GitIdentity(GitIdentityArgs),
    /// Log in to a Docker registry
    DockerLogin(DockerLoginArgs),
    /// Log out of a Docker registry
    DockerLogout(DockerLogoutArgs),
    /// Show Docker auth state (which registries are logged in)
    DockerShow(DockerShowArgs),
    /// Write an npm auth token to ~/.npmrc
    NpmToken(NpmTokenArgs),
    /// Print a status report of the dev environment
    Diagnostics(DiagnosticsArgs),
    /// Install core apps (git, gh, docker, obsidian) — idempotent
    InstallApps(InstallAppsArgs),
    /// Pick and persist the preferred shell
    PickShell(PickShellArgs),
    /// Pick and persist the preferred terminal
    PickTerminal(PickTerminalArgs),
    /// Install shell-experience packages (fzf, bat, eza, lazygit, etc.) — idempotent
    InstallPackages(InstallPackagesArgs),
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Doctor => doctor::gather()?,
        Commands::Repo { command } => match command {
            RepoCommands::Init(args) => repo::init(args)?,
        },
        Commands::Setup { command } => match command {
            SetupCommands::Detect => setup::run_detect()?,
            SetupCommands::Claude(args) => setup::run_claude(args)?,
            SetupCommands::GitIdentity(args) => setup::run_git_identity(&args)?,
            SetupCommands::DockerLogin(args) => setup::run_docker_login(&args)?,
            SetupCommands::DockerLogout(args) => setup::run_docker_logout(&args)?,
            SetupCommands::DockerShow(args) => setup::run_docker_show(&args)?,
            SetupCommands::NpmToken(args) => setup::run_npm_token(&args)?,
            SetupCommands::Diagnostics(args) => setup::run_diagnostics(&args)?,
            SetupCommands::InstallApps(args) => install::run_install_apps(args)?,
            SetupCommands::PickShell(args) => setup::run_pick_shell(args)?,
            SetupCommands::PickTerminal(args) => setup::run_pick_terminal(args)?,
            SetupCommands::InstallPackages(args) => setup::run_install_packages(&args)?,
        },
    }

    Ok(())
}
