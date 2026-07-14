/// `starbase setup diagnostics` — print a status report of the dev environment.
///
/// Covers: git identity (local + global), docker login state, npm token (redacted),
/// node/npm versions, shell info.  Read-only; no side effects.
use anyhow::Result;
use clap::Args;

use super::common::{git_config_get, on_path, probe_version};

// docker config
use serde::Deserialize;
use std::collections::HashMap;

// --- Clap argument struct ---

#[derive(Args, Debug)]
pub struct DiagnosticsArgs {}

// --- Helpers ---

fn git_local_identity() -> (Option<String>, Option<String>) {
    let name = git_config_get("--local", "user.name");
    let email = git_config_get("--local", "user.email");
    (name, email)
}

fn git_global_identity() -> (Option<String>, Option<String>) {
    let name = git_config_get("--global", "user.name");
    let email = git_config_get("--global", "user.email");
    (name, email)
}

#[derive(Debug, Deserialize, Default)]
struct DockerConfig {
    #[serde(default)]
    auths: HashMap<String, serde_json::Value>,
}

fn docker_registries() -> Vec<String> {
    let path = match dirs::home_dir() {
        Some(h) => h.join(".docker/config.json"),
        None => return vec![],
    };
    if !path.exists() {
        return vec![];
    }
    let text = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(_) => return vec![],
    };
    let cfg: DockerConfig = serde_json::from_str(&text).unwrap_or_default();
    let mut regs: Vec<String> = cfg.auths.into_keys().collect();
    regs.sort();
    regs
}

fn npm_token_status() -> String {
    let path = match dirs::home_dir() {
        Some(h) => h.join(".npmrc"),
        None => return "(home dir unknown)".to_string(),
    };
    if !path.exists() {
        return "(~/.npmrc not found)".to_string();
    }
    let text = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(_) => return "(could not read ~/.npmrc)".to_string(),
    };

    // Collect all :_authToken lines
    let tokens: Vec<String> = text
        .lines()
        .filter(|l| l.contains(":_authToken="))
        .map(|l| {
            // Extract the key and token parts, then redact the token.
            let (key_part, token_part) = l.split_once('=').unwrap_or((l, ""));
            let redacted = redact_token(token_part.trim());
            format!("{}={}", key_part.trim(), redacted)
        })
        .collect();

    if tokens.is_empty() {
        "(no :_authToken entries in ~/.npmrc)".to_string()
    } else {
        tokens.join(", ")
    }
}

/// Redact a token: show last 4 chars, mask the rest with `****`.
fn redact_token(token: &str) -> String {
    let chars: Vec<char> = token.chars().collect();
    if chars.len() <= 4 {
        return "****".to_string();
    }
    let visible: String = chars[chars.len() - 4..].iter().collect();
    format!("****{visible}")
}

fn shell_info() -> String {
    // Check common shells in order of preference
    let candidates = [
        ("pwsh", &["--version"][..]),
        ("zsh", &["--version"]),
        ("bash", &["--version"]),
        ("sh", &["--version"]),
    ];

    let mut found = Vec::new();
    for (shell, args) in &candidates {
        if on_path(shell) {
            let ver = probe_version(shell, args).unwrap_or_else(|| "?".to_string());
            found.push(format!("{shell} ({ver})"));
        }
    }

    if found.is_empty() {
        "(no recognized shell found)".to_string()
    } else {
        found.join(", ")
    }
}

// --- Entry point ---

pub fn run(_args: &DiagnosticsArgs) -> Result<()> {
    use std::fmt::Write;
    let mut out = String::new();

    let _ = writeln!(out, "=== starbase setup diagnostics ===");
    let _ = writeln!(out);

    // Git identity
    let _ = writeln!(out, "  Git identity:");
    let (local_name, local_email) = git_local_identity();
    let local_str = match (local_name, local_email) {
        (Some(n), Some(e)) => format!("{n} <{e}>"),
        (Some(n), None) => format!("{n} <email not set>"),
        (None, Some(e)) => format!("<name not set> {e}"),
        (None, None) => "(not set — not in a git repo or no local identity)".to_string(),
    };
    let _ = writeln!(out, "    local:   {local_str}");

    let (global_name, global_email) = git_global_identity();
    let global_str = match (global_name, global_email) {
        (Some(n), Some(e)) => format!("{n} <{e}>"),
        (Some(n), None) => format!("{n} <email not set>"),
        (None, Some(e)) => format!("<name not set> {e}"),
        (None, None) => "(not set)".to_string(),
    };
    let _ = writeln!(out, "    global:  {global_str}");

    let _ = writeln!(out);

    // Docker
    let _ = writeln!(out, "  Docker:");
    let registries = docker_registries();
    if registries.is_empty() {
        let _ = writeln!(out, "    (no registries logged in)");
    } else {
        for r in &registries {
            let _ = writeln!(out, "    \u{2713} {r}");
        }
    }

    let _ = writeln!(out);

    // npm token
    let _ = writeln!(out, "  npm token:");
    let _ = writeln!(out, "    {}", npm_token_status());

    let _ = writeln!(out);

    // Node / npm versions
    let _ = writeln!(out, "  Runtimes:");
    for (tool, args) in [
        ("node", &["--version"][..]),
        ("npm", &["--version"]),
        ("pnpm", &["--version"]),
    ] {
        if on_path(tool) {
            let ver = probe_version(tool, args).unwrap_or_else(|| "?".to_string());
            let _ = writeln!(out, "    \u{2713} {tool:<8} {ver}");
        } else {
            let _ = writeln!(out, "    \u{2717} {tool:<8} \u{2014}");
        }
    }

    let _ = writeln!(out);

    // Shell info
    let _ = writeln!(out, "  Shells:");
    let _ = writeln!(out, "    {}", shell_info());

    print!("{out}");
    Ok(())
}

// --- Tests ---

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redact_short_token() {
        assert_eq!(redact_token("abc"), "****");
    }

    #[test]
    fn redact_long_token() {
        let result = redact_token("abcdefgh");
        assert!(result.starts_with("****"));
        assert!(result.ends_with("efgh"));
    }

    #[test]
    fn redact_exactly_four_chars() {
        assert_eq!(redact_token("abcd"), "****");
    }
}
