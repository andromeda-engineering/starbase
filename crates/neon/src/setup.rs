use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

// ============================================================
// SECTION 1 — detect machine capabilities (neon setup detect)
// ============================================================

// --- Data types ---

#[derive(Debug, PartialEq, Eq)]
pub enum OsKind {
    Windows,
    Linux,
    MacOs,
    Unknown,
}

pub struct OsInfo {
    pub kind: OsKind,
    pub is_wsl: bool,
}

pub struct ToolPresence {
    pub name: String,
    pub found: bool,
    pub version: Option<String>,
}

pub struct CapabilityReport {
    pub os: OsInfo,
    pub arch: String,
    pub package_managers: Vec<String>,
    pub shells: Vec<String>,
    pub tools: Vec<ToolPresence>,
}

// --- Command helpers (mirror doctor.rs, not shared to avoid coupling) ---

#[cfg(windows)]
fn make_command(program: &str) -> Command {
    let mut cmd = Command::new("cmd");
    cmd.args(["/d", "/c", program]);
    cmd
}

#[cfg(not(windows))]
fn make_command(program: &str) -> Command {
    Command::new(program)
}

fn on_path(program: &str) -> bool {
    #[cfg(windows)]
    {
        Command::new("cmd")
            .args(["/d", "/c", "where", program])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
    #[cfg(not(windows))]
    {
        Command::new("which")
            .arg(program)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
}

fn probe_version(program: &str, args: &[&str]) -> Option<String> {
    match make_command(program).args(args).output() {
        Err(_) => None,
        Ok(out) => {
            if out.status.success() {
                // Prefer stdout; fall back to stderr (e.g. python --version on Python 2.x
                // and some older tools write the version string to stderr).
                let stdout = String::from_utf8_lossy(&out.stdout);
                let stderr = String::from_utf8_lossy(&out.stderr);
                let text = if stdout.trim().is_empty() {
                    stderr
                } else {
                    stdout
                };
                let line = text.trim().lines().next().unwrap_or("").trim().to_string();
                if line.is_empty() {
                    None
                } else {
                    Some(line)
                }
            } else {
                None
            }
        }
    }
}

// --- Detection logic ---

fn detect_os() -> OsInfo {
    let kind = if cfg!(target_os = "windows") {
        OsKind::Windows
    } else if cfg!(target_os = "linux") {
        OsKind::Linux
    } else if cfg!(target_os = "macos") {
        OsKind::MacOs
    } else {
        OsKind::Unknown
    };

    let is_wsl = if cfg!(target_os = "linux") {
        detect_wsl()
    } else {
        false
    };

    OsInfo { kind, is_wsl }
}

fn detect_wsl() -> bool {
    std::fs::read_to_string("/proc/version")
        .map(|contents| {
            let lower = contents.to_lowercase();
            lower.contains("microsoft") || lower.contains("wsl")
        })
        .unwrap_or(false)
}

fn detect_package_managers() -> Vec<String> {
    let candidates: &[&str] = if cfg!(target_os = "windows") {
        &["winget", "scoop", "choco"]
    } else if cfg!(target_os = "linux") {
        &["apt", "apt-get", "brew"]
    } else if cfg!(target_os = "macos") {
        &["brew", "port"]
    } else {
        &[]
    };

    candidates
        .iter()
        .filter(|&&name| on_path(name))
        .map(|&name| name.to_string())
        .collect()
}

fn detect_shells() -> Vec<String> {
    let candidates = ["pwsh", "powershell", "zsh", "bash", "fish"];
    candidates
        .iter()
        .filter(|&&name| on_path(name))
        .map(|&name| name.to_string())
        .collect()
}

fn detect_tools() -> Vec<ToolPresence> {
    let probes: &[(&str, &[&str])] = &[
        ("git", &["--version"]),
        ("gh", &["--version"]),
        ("docker", &["--version"]),
        ("node", &["--version"]),
        ("pnpm", &["--version"]),
        ("npm", &["--version"]),
        ("cargo", &["--version"]),
        ("rustup", &["--version"]),
        ("python", &["--version"]),
        ("python3", &["--version"]),
    ];

    probes
        .iter()
        .map(|&(name, args)| {
            let found = on_path(name);
            let version = if found {
                probe_version(name, args)
            } else {
                None
            };
            ToolPresence {
                name: name.to_string(),
                found,
                version,
            }
        })
        .collect()
}

// --- Public detection entry point ---

pub fn detect() -> Result<CapabilityReport> {
    let os = detect_os();
    let arch = std::env::consts::ARCH.to_string();
    let package_managers = detect_package_managers();
    let shells = detect_shells();
    let tools = detect_tools();

    Ok(CapabilityReport {
        os,
        arch,
        package_managers,
        shells,
        tools,
    })
}

// --- Formatting ---

fn os_label(os: &OsInfo) -> String {
    let kind = match os.kind {
        OsKind::Windows => "Windows",
        OsKind::Linux => "Linux",
        OsKind::MacOs => "macOS",
        OsKind::Unknown => "Unknown",
    };
    if os.is_wsl {
        format!("{kind} (WSL)")
    } else {
        format!("{kind} (not WSL)")
    }
}

pub fn format_report(report: &CapabilityReport) -> String {
    use std::fmt::Write;
    let mut s = String::new();

    let _ = writeln!(s, "=== neon setup detect ===");
    let _ = writeln!(s);
    let _ = writeln!(s, "  OS:       {}", os_label(&report.os));
    let _ = writeln!(s, "  Arch:     {}", report.arch);

    let _ = writeln!(s);
    let _ = writeln!(s, "  Package managers:");
    if report.package_managers.is_empty() {
        let _ = writeln!(s, "    (none detected)");
    } else {
        for pm in &report.package_managers {
            let _ = writeln!(s, "    \u{2713} {pm}");
        }
    }

    let _ = writeln!(s);
    let _ = writeln!(s, "  Shells:");
    if report.shells.is_empty() {
        let _ = writeln!(s, "    (none detected)");
    } else {
        for shell in &report.shells {
            let _ = writeln!(s, "    \u{2713} {shell}");
        }
    }

    let _ = writeln!(s);
    let _ = writeln!(s, "  Tools:");
    for tool in &report.tools {
        if tool.found {
            let ver = tool.version.as_deref().unwrap_or("");
            let _ = writeln!(s, "    \u{2713} {:<10} {ver}", tool.name);
        } else {
            let _ = writeln!(s, "    \u{2717} {:<10} \u{2014}", tool.name);
        }
    }

    s
}

pub fn print_report(report: &CapabilityReport) {
    print!("{}", format_report(report));
}

// --- Public entry point ---

pub fn run_detect() -> Result<()> {
    let report = detect()?;
    print_report(&report);
    Ok(())
}

// ============================================================
// SECTION 2 — Claude/agent bootstrap (neon setup claude)
// ============================================================

// --- Constants ---

const LOCAL_CONFIG_TEMPLATE: &str = "\
# Machine-local config (NOT committed)

- PLANNING_VAULT_ROOT: <PLANNING_VAULT_ROOT>
- WIKI_ROOT: <WIKI_ROOT>
- ISSUE_TRACKER: linear
- LINEAR_TEAM: <LINEAR_TEAM>
- LINEAR_PROJECT: <LINEAR_PROJECT>
- HANDOFF_TARGET: obsidian
- LOCAL_SKILLS_HOME: <LOCAL_SKILLS_HOME>
";

// --- Args ---

/// Arguments for `neon setup claude`.
#[derive(clap::Args, Debug)]
pub struct SetupClaudeArgs {
    /// Path to the claude-config repo clone (default: probed from common locations).
    #[arg(long, value_name = "PATH")]
    pub claude_config: Option<PathBuf>,

    /// Show what would happen without making changes.
    #[arg(long)]
    pub dry_run: bool,
}

// --- Home dir helper ---

/// Resolve the current user's home directory from environment variables.
///
/// Uses `USERPROFILE` on Windows (set by the OS for every user account),
/// falling back to `HOME` (standard on Unix / Git-Bash / WSL).  Returns an
/// error only when neither variable is set, which indicates a severely broken
/// environment rather than a recoverable condition.
fn home_dir() -> Result<PathBuf> {
    // USERPROFILE is the canonical Windows per-user home (e.g. C:\Users\alice).
    // HOME is set on Unix and in Git-Bash / WSL running on Windows.
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
        .context("could not determine home directory (USERPROFILE / HOME not set)")
}

// --- Claude-config path resolution ---

/// Try to locate the `claude-config` repository automatically.
///
/// Checks a set of conventional clone locations relative to the home directory
/// and returns the first one that is an existing directory.  Returns `None`
/// when none of the candidates exist so the caller can decide whether to
/// error out or continue (e.g. in dry-run mode).
fn find_claude_config(home: &Path) -> Option<PathBuf> {
    let candidates = [
        home.join(".claude-config"),
        home.join("claude-config"),
        home.join("projects").join("me").join("claude-config"),
    ];
    candidates.into_iter().find(|p| p.is_dir())
}

// --- Junction / symlink helpers ---

/// Decide whether a path is already a junction or symlink pointing at the
/// expected target.
///
/// On Windows, `std::fs::read_link` resolves the junction's reparse data, but
/// the result typically carries a `\\\\?\` long-path prefix that won't
/// byte-compare equal to a plain path.  We canonicalize *both* sides to
/// normalize them before comparing.
fn is_linked_to(link: &Path, expected_target: &Path) -> bool {
    // If the path is not a symlink / junction at all, return false immediately.
    let Ok(meta) = std::fs::symlink_metadata(link) else {
        return false;
    };
    if !meta.file_type().is_symlink() {
        return false;
    }
    // Try to read the link destination and canonicalize both sides.
    let Ok(actual) = std::fs::read_link(link) else {
        return false;
    };
    // Canonicalize the expected target (must exist for this to succeed).
    let Ok(canonical_expected) = std::fs::canonicalize(expected_target) else {
        return false;
    };
    // The actual link target might be relative (resolve against the link's parent) or
    // carry UNC prefixes.  Canonicalizing both sides normalizes them for comparison.
    let canonical_actual = if actual.is_absolute() {
        std::fs::canonicalize(&actual).unwrap_or(actual)
    } else {
        link.parent()
            .map(|parent| parent.join(&actual))
            .and_then(|abs| std::fs::canonicalize(&abs).ok())
            .unwrap_or(actual)
    };
    canonical_actual == canonical_expected
}

/// Create a directory junction (Windows) or symlink (Unix) from `link` to
/// `target`.
///
/// `link`   -- the new path to create (e.g. `~/.claude/skills`)
/// `target` -- the directory it should point at (e.g. `<claude-config>/skills`)
fn create_link(link: &Path, target: &Path) -> Result<()> {
    #[cfg(windows)]
    {
        let status = std::process::Command::new("cmd")
            .arg("/d")
            .arg("/c")
            .arg("mklink")
            .arg("/J")
            .arg(link.as_os_str())
            .arg(target.as_os_str())
            .status()
            .context("failed to run cmd /d /c mklink /J")?;
        if !status.success() {
            anyhow::bail!(
                "mklink /J failed (exit {}): {} -> {}",
                status.code().unwrap_or(-1),
                link.display(),
                target.display()
            );
        }
    }
    #[cfg(not(windows))]
    {
        std::os::unix::fs::symlink(target, link)
            .with_context(|| format!("symlink {} -> {}", link.display(), target.display()))?;
    }
    Ok(())
}

// --- Output helpers ---

fn print_step(dry_run: bool, icon: &str, label: &str, detail: &str) {
    if dry_run {
        println!("[dry-run] [{icon}] {label}: {detail}");
    } else {
        println!("[{icon}] {label}: {detail}");
    }
}

// --- Core operation: ensure junction ---

/// Ensure a junction/symlink at `link` points to `source`.
///
/// Possible outcomes:
/// - Already correct: skip with "(already exists)" message.
/// - Path exists but is a regular directory: warn and skip.
/// - Path does not exist: create the junction (or skip in dry-run).
fn ensure_junction(label: &str, link: &Path, source: &Path, dry_run: bool) -> Result<()> {
    let link_disp = link.display().to_string();
    let source_disp = source.display().to_string();

    if link.exists() || link.symlink_metadata().is_ok() {
        // Path already exists (as a junction, symlink, or directory).
        if is_linked_to(link, source) {
            print_step(
                dry_run,
                "✓",
                label,
                &format!("{link_disp} → {source_disp}  (already linked)"),
            );
            return Ok(());
        }
        // It's a real directory (or points somewhere else) -- warn and skip.
        print_step(
            dry_run,
            "!",
            label,
            &format!(
                "{link_disp} exists but is not a junction to the expected target -- skipping (manual review needed)"
            ),
        );
        return Ok(());
    }

    if dry_run {
        print_step(
            dry_run,
            "✓",
            label,
            &format!("{link_disp} → {source_disp}  (would create)"),
        );
        return Ok(());
    }

    create_link(link, source)?;
    print_step(
        dry_run,
        "✓",
        label,
        &format!("{link_disp} → {source_disp}  (created)"),
    );
    Ok(())
}

// --- Core operation: copy file if missing ---

fn ensure_file_copy(label: &str, src: &Path, dest: &Path, dry_run: bool) -> Result<()> {
    if !src.exists() {
        // Source not present in the claude-config repo -- nothing to copy.
        print_step(dry_run, "~", label, "skipped (source not in claude-config)");
        return Ok(());
    }
    if dest.exists() {
        print_step(dry_run, "~", label, "skipped (already exists)");
        return Ok(());
    }
    if dry_run {
        print_step(dry_run, "✓", label, "would copy");
        return Ok(());
    }
    std::fs::copy(src, dest)
        .with_context(|| format!("copying {} to {}", src.display(), dest.display()))?;
    print_step(dry_run, "✓", label, "copied");
    Ok(())
}

// --- Core operation: write local-config.md template if missing ---

fn ensure_local_config(dest: &Path, dry_run: bool) -> Result<()> {
    let label = "local-config.md";
    if dest.exists() {
        print_step(
            dry_run,
            "~",
            label,
            "skipped (already exists -- user manages this file)",
        );
        return Ok(());
    }
    if dry_run {
        print_step(dry_run, "✓", label, "would create (fill in your values)");
        return Ok(());
    }
    std::fs::write(dest, LOCAL_CONFIG_TEMPLATE)
        .with_context(|| format!("writing local-config.md to {}", dest.display()))?;
    print_step(dry_run, "✓", label, "created (fill in your values)");
    Ok(())
}

// --- Step 3: skill sync (Windows only) ---

#[cfg(windows)]
fn run_skill_sync(claude_config: &Path, dry_run: bool) -> Result<()> {
    let sync_script = claude_config.join("sync-skills.ps1");
    if !sync_script.exists() {
        print_step(
            dry_run,
            "~",
            "skill sync",
            "skipped (sync-skills.ps1 not found in claude-config)",
        );
        return Ok(());
    }
    if dry_run {
        print_step(dry_run, "✓", "skill sync", "would run sync-skills.ps1");
        return Ok(());
    }
    let status = std::process::Command::new("pwsh")
        .arg("-NonInteractive")
        .arg("-File")
        .arg(sync_script.as_os_str())
        .status()
        .context("failed to launch pwsh for sync-skills.ps1")?;
    if status.success() {
        print_step(dry_run, "✓", "skill sync", "ran sync-skills.ps1");
    } else {
        anyhow::bail!(
            "sync-skills.ps1 exited with code {}",
            status.code().unwrap_or(-1)
        );
    }
    Ok(())
}

#[cfg(not(windows))]
fn run_skill_sync(_claude_config: &Path, dry_run: bool) -> Result<()> {
    print_step(
        dry_run,
        "~",
        "skill sync",
        "skipped (no sync script on non-Windows)",
    );
    Ok(())
}

// --- Public entry point ---

/// Entry point for `neon setup claude`.
///
/// Bootstraps the machine-level Claude/agent environment idempotently.
/// In `--dry-run` mode, every step prints what would happen but makes no
/// changes and always exits successfully (even when `claude-config` cannot
/// be located).
pub fn run_claude(args: SetupClaudeArgs) -> Result<()> {
    let dry_run = args.dry_run;

    // --- Step 1: resolve claude-config path ---
    let home = home_dir()?;
    let claude_config = match args.claude_config {
        Some(p) => {
            if !p.is_dir() {
                let msg = format!(
                    "claude-config path \"{}\" is not an existing directory.\n\n`claude-config` is a personal repo that holds your shared Claude skills,\nagents, and global CLAUDE.md.  Clone it first:\n\n    git clone https://github.com/<you>/claude-config ~/claude-config\n\nThen re-run:  neon setup claude --claude-config ~/claude-config",
                    p.display()
                );
                if dry_run {
                    println!("[dry-run] [!] claude-config: {msg}");
                    return Ok(());
                }
                anyhow::bail!("{msg}");
            }
            p
        }
        None => match find_claude_config(&home) {
            Some(p) => {
                if dry_run {
                    println!("[dry-run] [~] claude-config: found at {}", p.display());
                } else {
                    println!("  Found claude-config at: {}", p.display());
                }
                p
            }
            None => {
                let msg = concat!(
                    "`claude-config` was not found at any of the default locations:",
                    "\n  ~/.claude-config",
                    "\n  ~/claude-config",
                    "\n  ~/projects/me/claude-config",
                    "\n",
                    "\n`claude-config` is a personal repo that holds your shared Claude skills,",
                    "\nagents, and global CLAUDE.md.  Clone it first:",
                    "\n",
                    "\n    git clone https://github.com/<you>/claude-config ~/claude-config",
                    "\n",
                    "\nThen re-run:  neon setup claude --claude-config ~/claude-config"
                );
                if dry_run {
                    println!("[dry-run] [!] claude-config: not found");
                    for line in msg.lines() {
                        println!("[dry-run]     {line}");
                    }
                    return Ok(());
                }
                anyhow::bail!("{msg}");
            }
        },
    };

    let dot_claude = home.join(".claude");

    // Ensure ~/.claude exists (no-op if already present).
    if !dry_run {
        std::fs::create_dir_all(&dot_claude)
            .with_context(|| format!("creating directory {}", dot_claude.display()))?;
    }

    // --- Step 2: junctions ---
    ensure_junction(
        "skills junction",
        &dot_claude.join("skills"),
        &claude_config.join("skills"),
        dry_run,
    )?;
    ensure_junction(
        "agents junction",
        &dot_claude.join("agents"),
        &claude_config.join("agents"),
        dry_run,
    )?;

    // --- Step 3: skill sync ---
    run_skill_sync(&claude_config, dry_run)?;

    // --- Step 4: CLAUDE.md ---
    ensure_file_copy(
        "CLAUDE.md",
        &claude_config.join("CLAUDE.md"),
        &dot_claude.join("CLAUDE.md"),
        dry_run,
    )?;

    // --- Step 5: settings.json ---
    ensure_file_copy(
        "settings.json",
        &claude_config.join("settings.json"),
        &dot_claude.join("settings.json"),
        dry_run,
    )?;

    // --- Step 6: local-config.md ---
    ensure_local_config(&dot_claude.join("local-config.md"), dry_run)?;

    Ok(())
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    // --- detect tests ---

    #[test]
    fn capability_report_has_os() {
        let os = detect_os();
        assert_ne!(os.kind, OsKind::Unknown, "OS kind should be detected, not Unknown");
    }

    #[test]
    fn arch_is_non_empty() {
        assert!(!std::env::consts::ARCH.is_empty(), "arch should be non-empty");
    }

    #[test]
    fn print_report_contains_headers() {
        let report = CapabilityReport {
            os: OsInfo { kind: OsKind::Linux, is_wsl: false },
            arch: "x86_64".to_string(),
            package_managers: vec![],
            shells: vec![],
            tools: vec![],
        };
        let output = format_report(&report);
        assert!(output.contains("OS:"), "output should contain 'OS:'");
        assert!(output.contains("Shells:"), "output should contain 'Shells:'");
        assert!(output.contains("Tools:"), "output should contain 'Tools:'");
    }

    // --- claude tests ---

    /// The template must contain all documented placeholder keys so callers can
    /// grep for them and verify the file needs filling in.
    #[test]
    fn local_config_template_contains_placeholders() {
        let expected_keys = [
            "PLANNING_VAULT_ROOT",
            "WIKI_ROOT",
            "ISSUE_TRACKER",
            "LINEAR_TEAM",
            "LINEAR_PROJECT",
            "HANDOFF_TARGET",
            "LOCAL_SKILLS_HOME",
        ];
        for key in expected_keys {
            assert!(
                LOCAL_CONFIG_TEMPLATE.contains(key),
                "LOCAL_CONFIG_TEMPLATE missing key: {key}"
            );
        }
    }

    /// `find_claude_config` should return None for a home directory that has
    /// none of the candidate subdirectories.
    #[test]
    fn auto_detect_returns_none_for_empty_home() {
        // Use a path that definitely does not contain claude-config clones.
        let fake_home = std::env::temp_dir().join("neon_test_empty_home_99999");
        // No directories are created, so all candidates are absent.
        let result = find_claude_config(&fake_home);
        assert!(
            result.is_none(),
            "expected None for fake home, got: {result:?}"
        );
    }

    /// `is_linked_to` must return false for a plain file that is not a symlink.
    #[test]
    fn is_linked_to_returns_false_for_plain_file() {
        // Use a path we know exists but is not a symlink (the current executable
        // is always a regular file on the target machine).
        let not_a_link = std::env::current_exe().expect("current_exe");
        assert!(
            !is_linked_to(&not_a_link, Path::new("C:/fake/target")),
            "is_linked_to should be false for a plain executable"
        );
    }

    /// `find_claude_config` finds a directory when it exists at the first
    /// candidate location (`<home>/.claude-config`).
    #[test]
    fn auto_detect_finds_dot_claude_config() {
        // Create a temporary directory structure that mimics the first candidate.
        let tmp = std::env::temp_dir().join(format!(
            "neon_test_find_cc_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.subsec_nanos())
                .unwrap_or(0)
        ));
        let candidate = tmp.join(".claude-config");
        std::fs::create_dir_all(&candidate).expect("create candidate dir");

        let found = find_claude_config(&tmp);
        // Clean up before asserting so a failure does not leave stale temp dirs.
        let _ = std::fs::remove_dir_all(&tmp);

        assert!(
            found.is_some(),
            "expected find_claude_config to find the candidate"
        );
        assert_eq!(found.unwrap(), candidate);
    }

    /// On Windows, a directory junction must be recognized by `is_linked_to`
    /// so that a second `neon setup claude` invocation is idempotent and does
    /// not attempt to recreate the junction.
    ///
    /// This is the canonical guard for the idempotency requirement: if
    /// `FileType::is_symlink()` returns false for junctions (it returns true
    /// per empirical check on Windows 11), the command would warn "exists but
    /// is not a junction" on every re-run instead of recognizing the existing
    /// link.
    #[cfg(windows)]
    #[test]
    fn is_linked_to_returns_true_for_existing_junction() {
        let tmp = std::env::temp_dir().join(format!(
            "neon_test_junction_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.subsec_nanos())
                .unwrap_or(0)
        ));
        let target = tmp.join("target");
        let link = tmp.join("link");

        std::fs::create_dir_all(&target).expect("create target dir");
        let out = std::process::Command::new("cmd")
            .arg("/d")
            .arg("/c")
            .arg("mklink")
            .arg("/J")
            .arg(link.as_os_str())
            .arg(target.as_os_str())
            .output()
            .expect("cmd /d /c mklink /J");

        // Clean up regardless of assertion result.
        let linked = is_linked_to(&link, &target);
        let _ = std::fs::remove_dir_all(&tmp);

        assert!(
            out.status.success(),
            "mklink /J failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(
            linked,
            "is_linked_to must return true for a correct Windows junction (idempotency)"
        );
    }
}
