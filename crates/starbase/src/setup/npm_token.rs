/// `starbase setup npm-token` — write an auth token to `~/.npmrc`.
///
/// Writes `//registry.npmjs.org/:_authToken=<token>` (or the equivalent line
/// for a custom registry).  Idempotent: replaces an existing matching line
/// rather than appending a duplicate.
use anyhow::{bail, Result};
use clap::Args;
use std::path::PathBuf;

use super::common::{dry_run_print, home_relative};

// --- Clap argument struct ---

#[derive(Args, Debug)]
pub struct NpmTokenArgs {
    /// Auth token to write
    #[arg(long)]
    pub token: String,

    /// Registry URL (default: https://registry.npmjs.org)
    #[arg(long, default_value = "https://registry.npmjs.org")]
    pub registry: String,

    /// Print planned changes without writing anything
    #[arg(long)]
    pub dry_run: bool,
}

// --- Helpers ---

fn npmrc_path() -> Option<PathBuf> {
    home_relative(".npmrc")
}

/// Convert a registry URL to the npmrc auth key.
///
/// `https://registry.npmjs.org` → `//registry.npmjs.org/:_authToken`
/// `https://npm.pkg.github.com` → `//npm.pkg.github.com/:_authToken`
pub(crate) fn registry_to_key(registry: &str) -> String {
    // Strip scheme (https:// or http://)
    let without_scheme = registry
        .strip_prefix("https://")
        .or_else(|| registry.strip_prefix("http://"))
        .unwrap_or(registry);

    // Ensure no trailing slash before appending the path segment
    let bare = without_scheme.trim_end_matches('/');
    format!("//{bare}/:_authToken")
}

/// Build the full line that should appear in `.npmrc`.
fn build_npmrc_line(registry: &str, token: &str) -> String {
    format!("{}={}", registry_to_key(registry), token)
}

/// Upsert the auth line in the npmrc file content (in-memory).
///
/// If a line matching `<key>=` already exists it is replaced; otherwise
/// the new line is appended.  Preserves all other lines unchanged.
pub(crate) fn upsert_npmrc_line(content: &str, registry: &str, token: &str) -> String {
    let key = registry_to_key(registry);
    let new_line = format!("{key}={token}");
    let prefix = format!("{key}=");

    let mut lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();
    let mut replaced = false;

    for line in &mut lines {
        if line.starts_with(&prefix) {
            *line = new_line.clone();
            replaced = true;
            break;
        }
    }

    if !replaced {
        lines.push(new_line);
    }

    // Preserve a trailing newline if the original had one.
    let trailing = if content.ends_with('\n') { "\n" } else { "" };
    format!("{}{}", lines.join("\n"), trailing)
}

// --- Entry point ---

pub fn run(args: &NpmTokenArgs) -> Result<()> {
    if args.token.is_empty() {
        bail!("--token cannot be empty");
    }

    let path = npmrc_path().ok_or_else(|| anyhow::anyhow!("Could not resolve home directory"))?;

    if args.dry_run {
        dry_run_print!(
            "upsert in {}: {}",
            path.display(),
            build_npmrc_line(&args.registry, "****")
        );
        return Ok(());
    }

    let existing = if path.exists() {
        std::fs::read_to_string(&path)?
    } else {
        String::new()
    };

    let updated = upsert_npmrc_line(&existing, &args.registry, &args.token);

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, &updated)?;

    println!("Written to {}", path.display());
    println!("  {}", build_npmrc_line(&args.registry, "****"));

    Ok(())
}

// --- Tests ---

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_to_key_strips_https() {
        assert_eq!(
            registry_to_key("https://registry.npmjs.org"),
            "//registry.npmjs.org/:_authToken"
        );
    }

    #[test]
    fn registry_to_key_strips_http() {
        assert_eq!(
            registry_to_key("http://registry.npmjs.org"),
            "//registry.npmjs.org/:_authToken"
        );
    }

    #[test]
    fn registry_to_key_no_scheme() {
        assert_eq!(
            registry_to_key("registry.npmjs.org"),
            "//registry.npmjs.org/:_authToken"
        );
    }

    #[test]
    fn registry_to_key_trailing_slash() {
        assert_eq!(
            registry_to_key("https://registry.npmjs.org/"),
            "//registry.npmjs.org/:_authToken"
        );
    }

    #[test]
    fn upsert_appends_when_absent() {
        let content = "other=value\n";
        let result = upsert_npmrc_line(content, "https://registry.npmjs.org", "mytoken");
        assert!(result.contains("//registry.npmjs.org/:_authToken=mytoken"));
        assert!(result.contains("other=value"));
    }

    #[test]
    fn upsert_replaces_existing_line() {
        let content = "//registry.npmjs.org/:_authToken=oldtoken\nother=value\n";
        let result = upsert_npmrc_line(content, "https://registry.npmjs.org", "newtoken");
        assert!(result.contains("//registry.npmjs.org/:_authToken=newtoken"));
        assert!(!result.contains("oldtoken"));
        assert_eq!(
            result.matches("_authToken").count(),
            1,
            "should not duplicate"
        );
    }

    #[test]
    fn upsert_preserves_trailing_newline() {
        let content = "foo=bar\n";
        let result = upsert_npmrc_line(content, "https://registry.npmjs.org", "tok");
        assert!(result.ends_with('\n'));
    }

    #[test]
    fn upsert_empty_content_produces_single_line() {
        let result = upsert_npmrc_line("", "https://registry.npmjs.org", "tok");
        assert_eq!(result, "//registry.npmjs.org/:_authToken=tok");
    }

    #[test]
    fn upsert_github_registry() {
        let content = "";
        let result = upsert_npmrc_line(content, "https://npm.pkg.github.com", "ghpat");
        assert!(result.contains("//npm.pkg.github.com/:_authToken=ghpat"));
    }
}
