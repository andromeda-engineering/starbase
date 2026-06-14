use anyhow::Result;
use std::process::Command;

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

// --- Tests ---

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capability_report_has_os() {
        let os = detect_os();
        assert_ne!(
            os.kind,
            OsKind::Unknown,
            "OS kind should be detected, not Unknown"
        );
    }

    #[test]
    fn arch_is_non_empty() {
        assert!(
            !std::env::consts::ARCH.is_empty(),
            "arch should be non-empty"
        );
    }

    #[test]
    fn print_report_contains_headers() {
        let report = CapabilityReport {
            os: OsInfo {
                kind: OsKind::Linux,
                is_wsl: false,
            },
            arch: "x86_64".to_string(),
            package_managers: vec![],
            shells: vec![],
            tools: vec![],
        };
        let output = format_report(&report);
        assert!(output.contains("OS:"), "output should contain 'OS:'");
        assert!(
            output.contains("Shells:"),
            "output should contain 'Shells:'"
        );
        assert!(output.contains("Tools:"), "output should contain 'Tools:'");
    }
}
