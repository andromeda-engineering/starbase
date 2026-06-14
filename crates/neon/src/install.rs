use anyhow::Result;
use clap::Args;
use std::process::Command;

use crate::setup::on_path;

// --- App enum ---

/// A core app that `neon setup install-apps` can install.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum App {
    Git,
    Gh,
    Docker,
    Obsidian,
}

impl App {
    /// All four core apps, in canonical order.
    pub fn all() -> &'static [App] {
        &[App::Git, App::Gh, App::Docker, App::Obsidian]
    }

    /// Short name used in CLI `--apps` list and output lines.
    pub fn name(self) -> &'static str {
        match self {
            App::Git => "git",
            App::Gh => "gh",
            App::Docker => "docker",
            App::Obsidian => "obsidian",
        }
    }

    /// Parse a comma-separated app list string, e.g. `"git,gh"`.
    pub fn parse_list(s: &str) -> Result<Vec<App>> {
        let mut apps = Vec::new();
        for token in s.split(',') {
            let t = token.trim();
            let app = match t {
                "git" => App::Git,
                "gh" => App::Gh,
                "docker" => App::Docker,
                "obsidian" => App::Obsidian,
                other => {
                    anyhow::bail!("unknown app '{other}'; valid choices: git, gh, docker, obsidian")
                }
            };
            apps.push(app);
        }
        if apps.is_empty() {
            anyhow::bail!("--apps list must not be empty");
        }
        Ok(apps)
    }
}

// --- Platform ---

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Platform {
    Windows,
    Linux,
    MacOs,
    Unknown,
}

fn current_platform() -> Platform {
    match std::env::consts::OS {
        "windows" => Platform::Windows,
        "linux" => Platform::Linux,
        "macos" => Platform::MacOs,
        _ => Platform::Unknown,
    }
}

// --- Install spec ---

/// A single install command (program + args) for a given app on a given platform.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallSpec {
    pub program: String,
    pub args: Vec<String>,
}

impl InstallSpec {
    fn new(program: impl Into<String>, args: &[&str]) -> Self {
        InstallSpec {
            program: program.into(),
            args: args.iter().map(|s| s.to_string()).collect(),
        }
    }

    /// Human-readable representation: `program arg1 arg2 …`
    pub fn display(&self) -> String {
        if self.args.is_empty() {
            self.program.clone()
        } else {
            format!("{} {}", self.program, self.args.join(" "))
        }
    }
}

/// Resolve the install spec for `app` on the given platform.
/// Returns `None` on macOS (not yet supported) and `Unknown`.
fn install_spec(app: App, platform: Platform) -> Option<InstallSpec> {
    match platform {
        Platform::Windows => Some(match app {
            App::Git => InstallSpec::new("winget", &["install", "--id", "Git.Git", "-e"]),
            App::Gh => InstallSpec::new("winget", &["install", "--id", "GitHub.cli", "-e"]),
            App::Docker => InstallSpec::new(
                "winget",
                &["install", "--id", "Docker.DockerDesktop", "-e"],
            ),
            App::Obsidian => {
                InstallSpec::new("winget", &["install", "--id", "Obsidian.Obsidian", "-e"])
            }
        }),
        Platform::Linux => Some(match app {
            App::Git => InstallSpec::new("sudo", &["apt-get", "install", "-y", "git"]),
            App::Gh => InstallSpec::new(
                "bash",
                &[
                    "-c",
                    "type -p curl >/dev/null || (sudo apt-get update && sudo apt-get install curl -y) && \
                     curl -fsSL https://cli.github.com/packages/githubcli-archive-keyring.gpg | \
                     sudo dd of=/usr/share/keyrings/githubcli-archive-keyring.gpg && \
                     sudo chmod go+r /usr/share/keyrings/githubcli-archive-keyring.gpg && \
                     echo \"deb [arch=$(dpkg --print-architecture) signed-by=/usr/share/keyrings/githubcli-archive-keyring.gpg] https://cli.github.com/packages stable main\" | \
                     sudo tee /etc/apt/sources.list.d/github-cli.list > /dev/null && \
                     sudo apt-get update && sudo apt-get install gh -y",
                ],
            ),
            App::Docker => InstallSpec::new(
                "bash",
                &[
                    "-c",
                    "curl -fsSL https://get.docker.com | sudo sh",
                ],
            ),
            App::Obsidian => InstallSpec::new("snap", &["install", "obsidian", "--classic"]),
        }),
        Platform::MacOs | Platform::Unknown => None,
    }
}

// --- Idempotency probe ---

/// Returns `true` if the given app appears to be already installed.
///
/// For CLI tools (`git`, `gh`, `docker`) we probe PATH.
/// For `obsidian` (a GUI app with no CLI), we query the platform's package
/// manager to check if the package is already registered.
pub(crate) fn is_installed(app: App, platform: Platform) -> bool {
    match app {
        App::Git => on_path("git"),
        App::Gh => on_path("gh"),
        App::Docker => on_path("docker"),
        App::Obsidian => is_obsidian_installed(platform),
    }
}

fn is_obsidian_installed(platform: Platform) -> bool {
    match platform {
        Platform::Windows => {
            // `winget list --id Obsidian.Obsidian` exits 0 and has output when installed.
            let output = Command::new("winget")
                .args(["list", "--id", "Obsidian.Obsidian"])
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::null())
                .output();
            match output {
                Ok(out) => {
                    let text = String::from_utf8_lossy(&out.stdout);
                    // winget list prints a header row + one row per match; if the ID
                    // actually appears in the output it is installed.
                    text.to_lowercase().contains("obsidian")
                }
                Err(_) => false,
            }
        }
        Platform::Linux => {
            // Try snap first, then flatpak.
            let snap_ok = Command::new("snap")
                .args(["list", "obsidian"])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .map(|s| s.success())
                .unwrap_or(false);
            if snap_ok {
                return true;
            }
            Command::new("flatpak")
                .args(["info", "md.obsidian.Obsidian"])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .map(|s| s.success())
                .unwrap_or(false)
        }
        Platform::MacOs | Platform::Unknown => false,
    }
}

// --- Plan types ---

/// The resolved action for one app.
#[derive(Debug, PartialEq, Eq)]
pub enum AppAction {
    /// App is already installed — no action needed.
    AlreadyInstalled,
    /// App will be installed using the given spec.
    WillInstall(InstallSpec),
    /// App was not requested in `--apps`.
    Skipped,
    /// Platform not yet supported for this app.
    PlatformUnsupported,
}

/// The plan for a single app.
#[derive(Debug)]
pub struct AppPlan {
    pub app: App,
    pub action: AppAction,
}

/// The full plan for all four apps.
#[derive(Debug)]
pub struct InstallPlan {
    pub items: Vec<AppPlan>,
}

// --- Plan derivation (pure, no I/O) ---

/// Build a plan without executing anything.
///
/// `requested` — the apps the user asked for (via `--apps`).
/// `platform` — the resolved platform.
/// `check_installed` — injectable probe so the pure function is testable without
///   real PATH lookups.
pub(crate) fn build_plan(
    requested: &[App],
    platform: Platform,
    check_installed: impl Fn(App, Platform) -> bool,
) -> InstallPlan {
    let items = App::all()
        .iter()
        .map(|&app| {
            if !requested.contains(&app) {
                return AppPlan {
                    app,
                    action: AppAction::Skipped,
                };
            }
            if check_installed(app, platform) {
                return AppPlan {
                    app,
                    action: AppAction::AlreadyInstalled,
                };
            }
            let action = match install_spec(app, platform) {
                Some(spec) => AppAction::WillInstall(spec),
                None => AppAction::PlatformUnsupported,
            };
            AppPlan { app, action }
        })
        .collect();

    InstallPlan { items }
}

// --- Formatting (pure) ---

/// Format the plan as a human-readable string.  Pure — suitable for testing.
pub fn format_plan(plan: &InstallPlan, dry_run: bool) -> String {
    use std::fmt::Write;
    let mut s = String::new();

    if dry_run {
        let _ = writeln!(
            s,
            "Installing core apps... (dry run — no changes will be made)"
        );
    } else {
        let _ = writeln!(s, "Installing core apps...");
    }

    for item in &plan.items {
        let name = item.app.name();
        match &item.action {
            AppAction::AlreadyInstalled => {
                let _ = writeln!(s, "  \u{2713} {name}: already installed");
            }
            AppAction::WillInstall(spec) => {
                if dry_run {
                    let _ = writeln!(s, "  \u{2192} {name}: would run: {}", spec.display());
                } else {
                    let _ = writeln!(s, "  \u{2192} {name}: installing via {}...", spec.program);
                }
            }
            AppAction::Skipped => {
                let _ = writeln!(s, "  ~ {name}: skipped (not in --apps list)");
            }
            AppAction::PlatformUnsupported => {
                let _ = writeln!(s, "  ! {name}: not yet supported on this platform");
            }
        }
    }

    s
}

// --- Execution ---

/// Result for one app after execution.
#[derive(Debug)]
pub enum AppResult {
    AlreadyInstalled,
    Installed,
    Failed(String),
    Skipped,
    PlatformUnsupported,
}

/// Execute the install plan (skipped in dry-run mode).
pub fn execute_plan(plan: &InstallPlan) -> Vec<(&App, AppResult)> {
    let mut results = Vec::new();

    for item in &plan.items {
        let result = match &item.action {
            AppAction::AlreadyInstalled => AppResult::AlreadyInstalled,
            AppAction::Skipped => AppResult::Skipped,
            AppAction::PlatformUnsupported => AppResult::PlatformUnsupported,
            AppAction::WillInstall(spec) => {
                let name = item.app.name();
                println!("  \u{2192} {name}: installing via {}...", spec.program);
                match run_install(spec) {
                    Ok(()) => {
                        println!("    done");
                        AppResult::Installed
                    }
                    Err(e) => {
                        eprintln!("    error: {e}");
                        AppResult::Failed(e.to_string())
                    }
                }
            }
        };
        results.push((&item.app, result));
    }

    results
}

fn run_install(spec: &InstallSpec) -> Result<()> {
    let status = Command::new(&spec.program)
        .args(&spec.args)
        .status()
        .map_err(|e| anyhow::anyhow!("failed to launch '{}': {e}", spec.program))?;

    if status.success() {
        Ok(())
    } else {
        let code = status
            .code()
            .map(|c| c.to_string())
            .unwrap_or_else(|| "?".to_string());
        anyhow::bail!("'{}' exited with code {code}", spec.program)
    }
}

// --- Summary ---

/// Print a summary of the install results.
pub fn print_summary(results: &[(&App, AppResult)]) {
    let installed: Vec<_> = results
        .iter()
        .filter(|(_, r)| matches!(r, AppResult::Installed))
        .map(|(a, _)| a.name())
        .collect();
    let failed: Vec<_> = results
        .iter()
        .filter(|(_, r)| matches!(r, AppResult::Failed(_)))
        .map(|(a, _)| a.name())
        .collect();

    println!();
    println!("Summary:");
    if installed.is_empty() && failed.is_empty() {
        println!("  Nothing to install.");
    } else {
        if !installed.is_empty() {
            println!("  Installed:  {}", installed.join(", "));
        }
        if !failed.is_empty() {
            println!("  Failed:     {}", failed.join(", "));
        }
    }
}

// --- CLI args ---

/// Arguments for `neon setup install-apps`.
#[derive(Args, Debug)]
pub struct InstallAppsArgs {
    /// Comma-separated list of apps to install (default: all).
    /// Valid values: git, gh, docker, obsidian.
    #[arg(long, value_name = "APPS")]
    pub apps: Option<String>,

    /// Print what would run without executing anything.
    #[arg(long)]
    pub dry_run: bool,

    /// Skip confirmation prompt.
    #[arg(long, short = 'y')]
    pub yes: bool,
}

// --- Public entry point ---

/// Entry point for `neon setup install-apps`.
pub fn run_install_apps(args: InstallAppsArgs) -> Result<()> {
    // Resolve the requested app list.
    let requested: Vec<App> = match &args.apps {
        Some(s) => App::parse_list(s)?,
        None => App::all().to_vec(),
    };

    let platform = current_platform();

    // Build the plan.
    let plan = build_plan(&requested, platform, is_installed);

    // Print the plan header.
    print!("{}", format_plan(&plan, args.dry_run));

    if args.dry_run {
        return Ok(());
    }

    // Confirmation prompt (skip when --yes).
    if !args.yes {
        // Determine if there is anything to actually do.
        let has_work = plan
            .items
            .iter()
            .any(|i| matches!(i.action, AppAction::WillInstall(_)));

        if has_work {
            print!("Install these apps? [y/N] ");
            use std::io::Write;
            std::io::stdout().flush()?;

            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;
            if !matches!(input.trim().to_lowercase().as_str(), "y" | "yes") {
                println!("Aborted.");
                return Ok(());
            }
        }
    }

    // Execute.
    let results = execute_plan(&plan);
    print_summary(&results);

    // Fail the command if any install failed.
    let any_failed = results
        .iter()
        .any(|(_, r)| matches!(r, AppResult::Failed(_)));
    if any_failed {
        anyhow::bail!("one or more apps failed to install");
    }

    Ok(())
}

// --- Tests ---

#[cfg(test)]
mod tests {
    use super::*;

    // A probe that pretends nothing is installed.
    fn nothing_installed(_app: App, _platform: Platform) -> bool {
        false
    }

    // A probe that pretends everything is installed.
    fn everything_installed(_app: App, _platform: Platform) -> bool {
        true
    }

    #[test]
    fn parse_list_all_four() {
        let apps = App::parse_list("git,gh,docker,obsidian").unwrap();
        assert_eq!(apps, vec![App::Git, App::Gh, App::Docker, App::Obsidian]);
    }

    #[test]
    fn parse_list_single() {
        let apps = App::parse_list("git").unwrap();
        assert_eq!(apps, vec![App::Git]);
    }

    #[test]
    fn parse_list_rejects_unknown() {
        assert!(App::parse_list("git,unknown").is_err());
    }

    #[test]
    fn parse_list_rejects_empty() {
        assert!(App::parse_list("").is_err());
    }

    #[test]
    fn dry_run_all_four_windows_shows_winget_commands() {
        let plan = build_plan(App::all(), Platform::Windows, nothing_installed);
        let output = format_plan(&plan, true);

        assert!(output.contains("dry run"), "should mention dry run");
        for app in App::all() {
            assert!(
                output.contains(app.name()),
                "output should mention app '{}'",
                app.name()
            );
        }
        // All four should show winget on Windows.
        assert!(
            output.contains("winget"),
            "Windows plan should mention winget"
        );
        // No app should be marked skipped.
        assert!(!output.contains("skipped"), "nothing should be skipped");
    }

    #[test]
    fn dry_run_git_only_skips_others() {
        let plan = build_plan(&[App::Git], Platform::Windows, nothing_installed);
        let output = format_plan(&plan, true);

        assert!(output.contains("git"), "should mention git");
        // Other apps should be skipped.
        assert!(
            output.contains("~ gh: skipped"),
            "gh should be skipped; got:\n{output}"
        );
        assert!(
            output.contains("~ docker: skipped"),
            "docker should be skipped"
        );
        assert!(
            output.contains("~ obsidian: skipped"),
            "obsidian should be skipped"
        );
    }

    #[test]
    fn already_installed_apps_show_checkmark() {
        let plan = build_plan(App::all(), Platform::Windows, everything_installed);
        let output = format_plan(&plan, false);

        for app in App::all() {
            let expected = format!("\u{2713} {}: already installed", app.name());
            assert!(
                output.contains(&expected),
                "expected '{}' in output; got:\n{output}",
                expected
            );
        }
    }

    #[test]
    fn macos_platform_shows_unsupported() {
        let plan = build_plan(App::all(), Platform::MacOs, nothing_installed);
        let output = format_plan(&plan, true);

        assert!(
            output.contains("not yet supported"),
            "macOS should report unsupported; got:\n{output}"
        );
    }

    #[test]
    fn install_spec_display_contains_program() {
        let spec = InstallSpec::new("winget", &["install", "--id", "Git.Git", "-e"]);
        let display = spec.display();
        assert!(display.starts_with("winget"));
        assert!(display.contains("Git.Git"));
    }

    #[test]
    fn linux_plan_has_apt_or_bash() {
        let plan = build_plan(App::all(), Platform::Linux, nothing_installed);
        let output = format_plan(&plan, true);
        // At minimum git uses apt; docker/gh use bash.
        assert!(
            output.contains("apt-get") || output.contains("bash"),
            "Linux plan should mention apt-get or bash; got:\n{output}"
        );
    }
}
