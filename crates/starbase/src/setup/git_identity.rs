/// `starbase setup git-identity` — manage git identity (name, email) in local or global scope.
///
/// Stored identities live in `~/.config/git/identities` as TOML.
/// Format: one [[identity]] table per entry, keyed by email.
///
/// ```toml
/// [[identity]]
/// email = "chris@example.com"
/// name = "Chris"
/// signing_key = ""   # optional GPG/SSH key
/// ```
use anyhow::{bail, Result};
use clap::{Args, ValueEnum};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Command;

use super::common::{dry_run_print, home_relative};

// --- Clap argument struct ---

#[derive(Args, Debug)]
pub struct GitIdentityArgs {
    /// Identity name (e.g. "Chris Coppola")
    #[arg(long)]
    pub name: Option<String>,

    /// Identity email
    #[arg(long)]
    pub email: Option<String>,

    /// Apply to local repo or global git config (default: local if in a git repo, else global)
    #[arg(long, value_enum)]
    pub scope: Option<IdentityScope>,

    /// List stored identities and exit
    #[arg(long, conflicts_with_all = ["name", "email", "scope", "dry_run"])]
    pub list: bool,

    /// Print planned changes without writing anything
    #[arg(long)]
    pub dry_run: bool,
}

#[derive(ValueEnum, Clone, Copy, Debug)]
pub enum IdentityScope {
    Local,
    Global,
}

// --- TOML model ---

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StoredIdentity {
    pub email: String,
    pub name: String,
    #[serde(default)]
    pub signing_key: String,
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct IdentityFile {
    #[serde(default)]
    identity: Vec<StoredIdentity>,
}

// --- Path resolution ---

fn identities_path() -> Option<PathBuf> {
    home_relative(".config/git/identities")
}

// --- File I/O ---

fn load_identities(path: &PathBuf) -> Result<IdentityFile> {
    if !path.exists() {
        return Ok(IdentityFile::default());
    }
    let text = std::fs::read_to_string(path)?;
    let parsed: IdentityFile = toml::from_str(&text)?;
    Ok(parsed)
}

fn save_identities(path: &PathBuf, file: &IdentityFile) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let text = toml::to_string_pretty(file)?;
    std::fs::write(path, text)?;
    Ok(())
}

// --- Git scope detection ---

fn is_inside_git_repo() -> bool {
    Command::new("git")
        .args(["rev-parse", "--is-inside-work-tree"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn resolve_scope(requested: Option<IdentityScope>) -> IdentityScope {
    match requested {
        Some(s) => s,
        None => {
            if is_inside_git_repo() {
                IdentityScope::Local
            } else {
                IdentityScope::Global
            }
        }
    }
}

fn scope_flag(scope: IdentityScope) -> &'static str {
    match scope {
        IdentityScope::Local => "--local",
        IdentityScope::Global => "--global",
    }
}

fn scope_label(scope: IdentityScope) -> &'static str {
    match scope {
        IdentityScope::Local => "local",
        IdentityScope::Global => "global",
    }
}

// --- Public entry point ---

pub fn run(args: &GitIdentityArgs) -> Result<()> {
    if args.list {
        return run_list();
    }

    let name = match &args.name {
        Some(n) => n.clone(),
        None => bail!("--name is required (use --list to list stored identities)"),
    };
    let email = match &args.email {
        Some(e) => e.clone(),
        None => bail!("--email is required (use --list to list stored identities)"),
    };

    let scope = resolve_scope(args.scope);

    if args.dry_run {
        dry_run_print!("git config {} user.name \"{}\"", scope_flag(scope), name);
        dry_run_print!("git config {} user.email \"{}\"", scope_flag(scope), email);
        let idpath = identities_path().unwrap_or_else(|| PathBuf::from("~/.config/git/identities"));
        dry_run_print!(
            "upsert identity email=\"{}\" name=\"{}\" in {}",
            email,
            name,
            idpath.display()
        );
        return Ok(());
    }

    // Write git config
    let flag = scope_flag(scope);
    let label = scope_label(scope);

    let name_ok = Command::new("git")
        .args(["config", flag, "user.name", &name])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    let email_ok = Command::new("git")
        .args(["config", flag, "user.email", &email])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    if !name_ok || !email_ok {
        bail!("git config failed — are you in a git repo for --scope local?");
    }

    println!("Set git identity ({label}): name=\"{name}\" email=\"{email}\"");

    // Upsert into identity store
    let idpath =
        identities_path().ok_or_else(|| anyhow::anyhow!("Could not resolve home directory"))?;

    let mut file = load_identities(&idpath)?;

    if let Some(existing) = file.identity.iter_mut().find(|i| i.email == email) {
        existing.name = name.clone();
    } else {
        file.identity.push(StoredIdentity {
            email: email.clone(),
            name: name.clone(),
            signing_key: String::new(),
        });
    }

    save_identities(&idpath, &file)?;
    println!("Stored identity in {}", idpath.display());

    Ok(())
}

fn run_list() -> Result<()> {
    let idpath =
        identities_path().ok_or_else(|| anyhow::anyhow!("Could not resolve home directory"))?;

    let file = load_identities(&idpath)?;

    if file.identity.is_empty() {
        println!("No stored identities found in {}", idpath.display());
        return Ok(());
    }

    println!("Stored identities ({}):", idpath.display());
    for id in &file.identity {
        if id.signing_key.is_empty() {
            println!("  {} <{}>", id.name, id.email);
        } else {
            println!("  {} <{}> [signing: {}]", id.name, id.email, id.signing_key);
        }
    }

    Ok(())
}

// --- Tests ---

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;

    fn temp_path(name: &str) -> PathBuf {
        env::temp_dir().join(format!("starbase-test-{}-{}", std::process::id(), name))
    }

    #[test]
    fn identity_toml_roundtrip() {
        let path = temp_path("identities-roundtrip");
        let _ = fs::remove_file(&path); // clean up from any prior run

        let file = IdentityFile {
            identity: vec![
                StoredIdentity {
                    email: "chris@example.com".to_string(),
                    name: "Chris".to_string(),
                    signing_key: String::new(),
                },
                StoredIdentity {
                    email: "work@corp.example".to_string(),
                    name: "Chris Corp".to_string(),
                    signing_key: "ABC123".to_string(),
                },
            ],
        };
        save_identities(&path, &file).unwrap();
        let loaded = load_identities(&path).unwrap();
        let _ = fs::remove_file(&path);

        assert_eq!(loaded.identity.len(), 2);
        assert_eq!(loaded.identity[0].email, "chris@example.com");
        assert_eq!(loaded.identity[1].signing_key, "ABC123");
    }

    #[test]
    fn load_identities_missing_file_returns_empty() {
        let path = temp_path("identities-missing");
        let _ = fs::remove_file(&path);
        let file = load_identities(&path).unwrap();
        assert!(file.identity.is_empty());
    }

    #[test]
    fn upsert_updates_existing_name() {
        let path = temp_path("identities-upsert");
        let _ = fs::remove_file(&path);

        let initial = IdentityFile {
            identity: vec![StoredIdentity {
                email: "chris@example.com".to_string(),
                name: "Old Name".to_string(),
                signing_key: String::new(),
            }],
        };
        save_identities(&path, &initial).unwrap();

        let mut file = load_identities(&path).unwrap();
        if let Some(existing) = file
            .identity
            .iter_mut()
            .find(|i| i.email == "chris@example.com")
        {
            existing.name = "New Name".to_string();
        }
        save_identities(&path, &file).unwrap();

        let reloaded = load_identities(&path).unwrap();
        let _ = fs::remove_file(&path);

        assert_eq!(reloaded.identity.len(), 1);
        assert_eq!(reloaded.identity[0].name, "New Name");
    }
}
