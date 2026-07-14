/// `starbase setup docker-login` / `docker-logout` / `docker-show`
///
/// - `docker-login`: invoke `docker login [--username <u>] <registry>`
/// - `docker-logout`: invoke `docker logout <registry>`
/// - `docker-show`: read `~/.docker/config.json` and print logged-in registries
use anyhow::{bail, Result};
use clap::Args;
use serde::Deserialize;
use std::collections::HashMap;

use super::common::{dry_run_print, home_relative, on_path, run_interactive};

// --- Clap argument structs ---

#[derive(Args, Debug)]
pub struct DockerLoginArgs {
    /// Docker registry hostname (default: docker.io)
    #[arg(long, default_value = "docker.io")]
    pub registry: String,

    /// Username (optional — docker will prompt if omitted)
    #[arg(long)]
    pub username: Option<String>,

    /// Print planned actions without running docker login
    #[arg(long)]
    pub dry_run: bool,
}

#[derive(Args, Debug)]
pub struct DockerLogoutArgs {
    /// Docker registry hostname (default: docker.io)
    #[arg(long, default_value = "docker.io")]
    pub registry: String,

    /// Print planned actions without running docker logout
    #[arg(long)]
    pub dry_run: bool,
}

// docker-show has no meaningful args beyond --help
#[derive(Args, Debug)]
pub struct DockerShowArgs {}

// --- Docker config JSON model (partial) ---

#[derive(Debug, Deserialize, Default)]
struct DockerConfig {
    #[serde(default)]
    auths: HashMap<String, serde_json::Value>,
}

// --- Path resolution ---

fn docker_config_path() -> Option<std::path::PathBuf> {
    home_relative(".docker/config.json")
}

// --- Helpers ---

fn load_docker_config() -> DockerConfig {
    let path = match docker_config_path() {
        Some(p) => p,
        None => return DockerConfig::default(),
    };
    if !path.exists() {
        return DockerConfig::default();
    }
    let text = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(_) => return DockerConfig::default(),
    };
    serde_json::from_str(&text).unwrap_or_default()
}

// --- Entry points ---

pub fn run_login(args: &DockerLoginArgs) -> Result<()> {
    if !on_path("docker") {
        bail!("docker is not on PATH — install Docker first");
    }

    if args.dry_run {
        match &args.username {
            Some(u) => dry_run_print!("docker login --username {} {}", u, args.registry),
            None => dry_run_print!(
                "docker login {} (will prompt for credentials)",
                args.registry
            ),
        }
        return Ok(());
    }

    let mut cmd_args: Vec<&str> = Vec::new();
    if let Some(u) = &args.username {
        cmd_args.push("--username");
        cmd_args.push(u.as_str());
    }
    cmd_args.push(&args.registry);

    // Use inherited stdio so docker can prompt for the password.
    let ok = run_interactive("docker", &{
        let mut full = vec!["login"];
        full.extend(cmd_args.iter().copied());
        full
    });

    if !ok {
        bail!("docker login failed");
    }

    Ok(())
}

pub fn run_logout(args: &DockerLogoutArgs) -> Result<()> {
    if !on_path("docker") {
        bail!("docker is not on PATH");
    }

    if args.dry_run {
        dry_run_print!("docker logout {}", args.registry);
        return Ok(());
    }

    let ok = run_interactive("docker", &["logout", &args.registry]);
    if !ok {
        bail!("docker logout failed");
    }

    Ok(())
}

pub fn run_show(_args: &DockerShowArgs) -> Result<()> {
    let config = load_docker_config();

    let path =
        docker_config_path().unwrap_or_else(|| std::path::PathBuf::from("~/.docker/config.json"));
    println!("Docker auth state ({})", path.display());

    if config.auths.is_empty() {
        println!("  (no registries logged in)");
        return Ok(());
    }

    let mut registries: Vec<&String> = config.auths.keys().collect();
    registries.sort();
    for registry in registries {
        println!("  \u{2713} {registry}");
    }

    Ok(())
}

// --- Tests ---

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_docker_config_missing_file_returns_default() {
        // This uses the real home dir path. If the file is present it loads fine;
        // if absent it should return an empty default — not panic.
        let config = load_docker_config();
        // The auths map may or may not be populated; what matters is no panic.
        let _ = config.auths.len();
    }

    #[test]
    fn docker_config_parse_empty_auths() {
        let json = r#"{"auths": {}}"#;
        let cfg: DockerConfig = serde_json::from_str(json).unwrap();
        assert!(cfg.auths.is_empty());
    }

    #[test]
    fn docker_config_parse_with_registry() {
        let json = r#"{"auths": {"docker.io": {"auth": "dXNlcjpwYXNz"}}}"#;
        let cfg: DockerConfig = serde_json::from_str(json).unwrap();
        assert!(cfg.auths.contains_key("docker.io"));
    }

    #[test]
    fn docker_config_missing_auths_key_defaults_empty() {
        let json = r#"{"credsStore": "desktop"}"#;
        let cfg: DockerConfig = serde_json::from_str(json).unwrap();
        assert!(cfg.auths.is_empty());
    }
}
