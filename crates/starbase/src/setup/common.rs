/// Shared helpers used across setup subcommands.
use std::path::PathBuf;

use dirs::home_dir;

/// Resolve a path relative to the user's home directory.
///
/// All setup commands use literal `~/...` paths (e.g. `~/.config/git/identities`)
/// so they stay compatible across tools regardless of platform XDG conventions.
pub(crate) fn home_relative(rel: &str) -> Option<PathBuf> {
    home_dir().map(|h| h.join(rel))
}

/// Print a dry-run notice.
macro_rules! dry_run_print {
    ($($arg:tt)*) => {{
        println!("[dry-run] {}", format!($($arg)*));
    }};
}
pub(crate) use dry_run_print;

/// Run a command and inherit stdio so interactive prompts work.
///
/// Returns `true` if the process exited successfully.
#[allow(dead_code)]
pub(crate) fn run_interactive(program: &str, args: &[&str]) -> bool {
    // On Windows, route through `cmd /c` to resolve .cmd/.bat shims.
    #[cfg(windows)]
    let status = std::process::Command::new("cmd")
        .args(["/d", "/c", program])
        .args(args)
        .status();

    #[cfg(not(windows))]
    let status = std::process::Command::new(program).args(args).status();

    status.map(|s| s.success()).unwrap_or(false)
}

/// Probe whether a program is on PATH.
pub(crate) fn on_path(program: &str) -> bool {
    #[cfg(windows)]
    {
        std::process::Command::new("cmd")
            .args(["/d", "/c", "where", program])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
    #[cfg(not(windows))]
    {
        std::process::Command::new("which")
            .arg(program)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
}

/// Probe a program's version string.
pub(crate) fn probe_version(program: &str, args: &[&str]) -> Option<String> {
    #[cfg(windows)]
    let mut cmd = {
        let mut c = std::process::Command::new("cmd");
        c.args(["/d", "/c", program]);
        c
    };
    #[cfg(not(windows))]
    let mut cmd = std::process::Command::new(program);

    match cmd.args(args).output() {
        Err(_) => None,
        Ok(out) => {
            if out.status.success() {
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

/// Run `git config <scope> <key>` and return the trimmed output, if any.
pub(crate) fn git_config_get(scope: &str, key: &str) -> Option<String> {
    let output = std::process::Command::new("git")
        .args(["config", scope, "--get", key])
        .output()
        .ok()?;

    if output.status.success() {
        let val = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if val.is_empty() {
            None
        } else {
            Some(val)
        }
    } else {
        None
    }
}
