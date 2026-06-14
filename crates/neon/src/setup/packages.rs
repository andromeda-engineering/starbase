/// `neon setup install-packages` — idempotent shell-experience package installer.
///
/// Mirrors the structure of `crate::install` (InstallSpec/build_plan/execute_plan)
/// but covers the full set of shell-experience tools across Windows, Linux, and macOS.
use anyhow::Result;
use clap::Args;
use std::process::Command;

use crate::install::{InstallSpec, Platform};
use crate::setup::on_path;

// ---------------------------------------------------------------------------
// Package enum
// ---------------------------------------------------------------------------

/// A shell-experience package that `neon setup install-packages` can install.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Package {
    Zsh,
    OhMyPosh,
    OhMyZsh,
    PoshGit,
    Lazygit,
    Fzf,
    Bat,
    Zoxide,
    Eza,
    GitDelta,
    Ripgrep,
    Fd,
}

impl Package {
    /// All packages in canonical order.
    pub fn all() -> &'static [Package] {
        &[
            Package::Zsh,
            Package::OhMyPosh,
            Package::OhMyZsh,
            Package::PoshGit,
            Package::Lazygit,
            Package::Fzf,
            Package::Bat,
            Package::Zoxide,
            Package::Eza,
            Package::GitDelta,
            Package::Ripgrep,
            Package::Fd,
        ]
    }

    /// Short name used for `--skip` matching and output lines.
    pub fn name(self) -> &'static str {
        match self {
            Package::Zsh => "zsh",
            Package::OhMyPosh => "oh-my-posh",
            Package::OhMyZsh => "oh-my-zsh",
            Package::PoshGit => "posh-git",
            Package::Lazygit => "lazygit",
            Package::Fzf => "fzf",
            Package::Bat => "bat",
            Package::Zoxide => "zoxide",
            Package::Eza => "eza",
            Package::GitDelta => "git-delta",
            Package::Ripgrep => "ripgrep",
            Package::Fd => "fd",
        }
    }

    /// User-facing display name.
    pub fn display_name(self) -> &'static str {
        match self {
            Package::Zsh => "zsh",
            Package::OhMyPosh => "Oh My Posh",
            Package::OhMyZsh => "Oh My Zsh",
            Package::PoshGit => "posh-git",
            Package::Lazygit => "lazygit",
            Package::Fzf => "fzf",
            Package::Bat => "bat",
            Package::Zoxide => "zoxide",
            Package::Eza => "eza",
            Package::GitDelta => "git-delta (delta)",
            Package::Ripgrep => "ripgrep (rg)",
            Package::Fd => "fd",
        }
    }
}

// ---------------------------------------------------------------------------
// Platform detection (local copy — mirrors install.rs private helper)
// ---------------------------------------------------------------------------

fn current_platform() -> Platform {
    match std::env::consts::OS {
        "windows" => Platform::Windows,
        "linux" => Platform::Linux,
        "macos" => Platform::MacOs,
        _ => Platform::Unknown,
    }
}

// ---------------------------------------------------------------------------
// Idempotency probes
// ---------------------------------------------------------------------------

/// Returns `true` if `pkg` is already installed on the current machine.
///
/// Each package has a bespoke probe because some tools are not PATH binaries
/// (PoshGit is a PS module, OhMyZsh is a directory) and some ship under
/// alternative binary names on certain distros (bat -> batcat, fd -> fdfind).
pub(crate) fn is_installed(pkg: Package, _platform: Platform) -> bool {
    match pkg {
        Package::Zsh => on_path("zsh"),
        Package::OhMyPosh => on_path("oh-my-posh"),
        Package::OhMyZsh => {
            // oh-my-zsh has no binary; it lives in ~/.oh-my-zsh
            dirs::home_dir()
                .map(|h| h.join(".oh-my-zsh").is_dir())
                .unwrap_or(false)
        }
        Package::PoshGit => {
            // posh-git is a PowerShell module, not a PATH binary.
            // Probe with Get-Module -ListAvailable posh-git.
            #[cfg(windows)]
            {
                let out = Command::new("powershell")
                    .args([
                        "-NoProfile",
                        "-NonInteractive",
                        "-Command",
                        "if (Get-Module -ListAvailable -Name posh-git) { exit 0 } else { exit 1 }",
                    ])
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .status();
                out.map(|s| s.success()).unwrap_or(false)
            }
            #[cfg(not(windows))]
            {
                false // posh-git is Windows-only
            }
        }
        Package::Lazygit => on_path("lazygit"),
        Package::Fzf => on_path("fzf"),
        Package::Bat => {
            // Ubuntu ships bat as `batcat`; upstream ships as `bat`
            on_path("bat") || on_path("batcat")
        }
        Package::Zoxide => on_path("zoxide"),
        Package::Eza => on_path("eza"),
        Package::GitDelta => on_path("delta"),
        Package::Ripgrep => on_path("rg"),
        Package::Fd => {
            // Ubuntu ships fd as `fdfind`; upstream ships as `fd`
            on_path("fd") || on_path("fdfind")
        }
    }
}

// ---------------------------------------------------------------------------
// Install spec builder
// ---------------------------------------------------------------------------

/// A small helper that constructs an `InstallSpec` using public fields.
/// (InstallSpec::new is private to install.rs.)
fn spec(program: &str, args: &[&str]) -> InstallSpec {
    InstallSpec {
        program: program.to_string(),
        args: args.iter().map(|s| s.to_string()).collect(),
    }
}

/// Resolve the install spec for `pkg` on the given platform.
///
/// Returns `None` when the package is not applicable on this platform
/// (e.g. zsh on Windows, posh-git on Linux/macOS) or the platform is Unknown.
pub(crate) fn install_spec(pkg: Package, platform: Platform) -> Option<InstallSpec> {
    match platform {
        Platform::Windows => windows_spec(pkg),
        Platform::Linux => linux_spec(pkg),
        Platform::MacOs => macos_spec(pkg),
        Platform::Unknown => None,
    }
}

fn windows_spec(pkg: Package) -> Option<InstallSpec> {
    match pkg {
        Package::Zsh => None,     // not applicable on Windows
        Package::OhMyZsh => None, // not applicable on Windows
        Package::OhMyPosh => Some(spec(
            "winget",
            &[
                "install",
                "--id",
                "JanDeDobbeleer.OhMyPosh",
                "-e",
                "--accept-source-agreements",
                "--accept-package-agreements",
            ],
        )),
        Package::PoshGit => Some(spec(
            "powershell",
            &[
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "Install-Module posh-git -Scope CurrentUser -Force",
            ],
        )),
        Package::Lazygit => Some(spec(
            "winget",
            &[
                "install",
                "--id",
                "JesseDuffield.lazygit",
                "-e",
                "--accept-source-agreements",
                "--accept-package-agreements",
            ],
        )),
        Package::Fzf => Some(spec(
            "winget",
            &[
                "install",
                "--id",
                "junegunn.fzf",
                "-e",
                "--accept-source-agreements",
                "--accept-package-agreements",
            ],
        )),
        Package::Bat => Some(spec(
            "winget",
            &[
                "install",
                "--id",
                "sharkdp.bat",
                "-e",
                "--accept-source-agreements",
                "--accept-package-agreements",
            ],
        )),
        Package::Zoxide => Some(spec(
            "winget",
            &[
                "install",
                "--id",
                "ajeetdsouza.zoxide",
                "-e",
                "--accept-source-agreements",
                "--accept-package-agreements",
            ],
        )),
        Package::Eza => Some(spec(
            "winget",
            &[
                "install",
                "--id",
                "eza-community.eza",
                "-e",
                "--accept-source-agreements",
                "--accept-package-agreements",
            ],
        )),
        Package::GitDelta => Some(spec(
            "winget",
            &[
                "install",
                "--id",
                "dandavison.delta",
                "-e",
                "--accept-source-agreements",
                "--accept-package-agreements",
            ],
        )),
        Package::Ripgrep => Some(spec(
            "winget",
            &[
                "install",
                "--id",
                "BurntSushi.ripgrep.MSVC",
                "-e",
                "--accept-source-agreements",
                "--accept-package-agreements",
            ],
        )),
        Package::Fd => Some(spec(
            "winget",
            &[
                "install",
                "--id",
                "sharkdp.fd",
                "-e",
                "--accept-source-agreements",
                "--accept-package-agreements",
            ],
        )),
    }
}

fn linux_spec(pkg: Package) -> Option<InstallSpec> {
    match pkg {
        Package::Zsh => Some(spec("sudo", &["apt-get", "install", "-y", "zsh"])),
        Package::OhMyPosh => Some(spec(
            "bash",
            &["-c", "curl -s https://ohmyposh.dev/install.sh | bash -s"],
        )),
        Package::OhMyZsh => Some(spec(
            "sh",
            &[
                "-c",
                r#"sh -c "$(curl -fsSL https://raw.githubusercontent.com/ohmyzsh/ohmyzsh/master/tools/install.sh)" "" --unattended"#,
            ],
        )),
        Package::PoshGit => None, // Linux: not applicable
        Package::Lazygit => Some(spec(
            "bash",
            &[
                "-c",
                "LAZYGIT_VERSION=$(curl -s \"https://api.github.com/repos/jesseduffield/lazygit/releases/latest\" | grep -Po '\"tag_name\": \"v\\K[^\"]*') && \
                 curl -Lo /tmp/lazygit.tar.gz \"https://github.com/jesseduffield/lazygit/releases/latest/download/lazygit_${LAZYGIT_VERSION}_Linux_x86_64.tar.gz\" && \
                 tar xf /tmp/lazygit.tar.gz -C /tmp lazygit && \
                 sudo install /tmp/lazygit /usr/local/bin",
            ],
        )),
        Package::Fzf => Some(spec("sudo", &["apt-get", "install", "-y", "fzf"])),
        Package::Bat => Some(spec("sudo", &["apt-get", "install", "-y", "bat"])),
        Package::Zoxide => Some(spec(
            "sh",
            &[
                "-c",
                "curl -sSfL https://raw.githubusercontent.com/ajeetdsouza/zoxide/main/install.sh | sh",
            ],
        )),
        Package::Eza => Some(spec("sudo", &["apt-get", "install", "-y", "eza"])),
        Package::GitDelta => Some(spec(
            "sudo",
            &["apt-get", "install", "-y", "git-delta"],
        )),
        Package::Ripgrep => Some(spec(
            "sudo",
            &["apt-get", "install", "-y", "ripgrep"],
        )),
        Package::Fd => Some(spec("sudo", &["apt-get", "install", "-y", "fd-find"])),
    }
}

fn macos_spec(pkg: Package) -> Option<InstallSpec> {
    match pkg {
        Package::Zsh => Some(spec("brew", &["install", "zsh"])),
        Package::OhMyPosh => Some(spec(
            "brew",
            &["install", "jandedobbeleer/oh-my-posh/oh-my-posh"],
        )),
        Package::OhMyZsh => Some(spec(
            "sh",
            &[
                "-c",
                r#"sh -c "$(curl -fsSL https://raw.githubusercontent.com/ohmyzsh/ohmyzsh/master/tools/install.sh)" "" --unattended"#,
            ],
        )),
        Package::PoshGit => None, // macOS: not applicable
        Package::Lazygit => Some(spec("brew", &["install", "lazygit"])),
        Package::Fzf => Some(spec("brew", &["install", "fzf"])),
        Package::Bat => Some(spec("brew", &["install", "bat"])),
        Package::Zoxide => Some(spec("brew", &["install", "zoxide"])),
        Package::Eza => Some(spec("brew", &["install", "eza"])),
        Package::GitDelta => Some(spec("brew", &["install", "git-delta"])),
        Package::Ripgrep => Some(spec("brew", &["install", "ripgrep"])),
        Package::Fd => Some(spec("brew", &["install", "fd"])),
    }
}

// ---------------------------------------------------------------------------
// Plan types
// ---------------------------------------------------------------------------

/// The resolved action for one package.
#[derive(Debug, PartialEq, Eq)]
pub enum PkgAction {
    /// Package is already installed — no action needed.
    AlreadyInstalled,
    /// Package will be installed using the given spec.
    WillInstall(InstallSpec),
    /// Package was excluded via `--skip`.
    Skipped,
    /// Package is not applicable on this platform (e.g. zsh on Windows).
    NotApplicable,
    /// Platform is not recognised.
    PlatformUnsupported,
}

/// The plan for a single package.
#[derive(Debug)]
pub struct PkgPlan {
    pub pkg: Package,
    pub action: PkgAction,
}

/// The full plan for all packages.
#[derive(Debug)]
pub struct PackageInstallPlan {
    pub items: Vec<PkgPlan>,
}

// ---------------------------------------------------------------------------
// Plan derivation (pure — no I/O)
// ---------------------------------------------------------------------------

/// Build an install plan without executing anything.
///
/// `skip`            — package names (lowercased) to exclude.
/// `platform`        — resolved platform.
/// `check_installed` — injectable probe for testability.
pub(crate) fn build_plan(
    skip: &[String],
    platform: Platform,
    check_installed: impl Fn(Package, Platform) -> bool,
) -> PackageInstallPlan {
    let items = Package::all()
        .iter()
        .map(|&pkg| {
            if skip.iter().any(|s| s == pkg.name()) {
                return PkgPlan {
                    pkg,
                    action: PkgAction::Skipped,
                };
            }
            if check_installed(pkg, platform) {
                return PkgPlan {
                    pkg,
                    action: PkgAction::AlreadyInstalled,
                };
            }
            if platform == Platform::Unknown {
                return PkgPlan {
                    pkg,
                    action: PkgAction::PlatformUnsupported,
                };
            }
            let action = match install_spec(pkg, platform) {
                Some(s) => PkgAction::WillInstall(s),
                None => PkgAction::NotApplicable,
            };
            PkgPlan { pkg, action }
        })
        .collect();

    PackageInstallPlan { items }
}

// ---------------------------------------------------------------------------
// Formatting (pure — suitable for tests)
// ---------------------------------------------------------------------------

/// Format the plan as a human-readable string.
pub(crate) fn format_plan(plan: &PackageInstallPlan, dry_run: bool) -> String {
    use std::fmt::Write;
    let mut s = String::new();

    if dry_run {
        let _ = writeln!(
            s,
            "Installing shell-experience packages... (dry run \u{2014} no changes will be made)"
        );
    } else {
        let _ = writeln!(s, "Installing shell-experience packages...");
    }

    for item in &plan.items {
        let name = item.pkg.display_name();
        match &item.action {
            PkgAction::AlreadyInstalled => {
                let _ = writeln!(s, "  \u{2713} {name}: already installed");
            }
            PkgAction::WillInstall(spec) => {
                if dry_run {
                    let _ = writeln!(s, "  \u{2192} {name}: would run: {}", spec.display());
                } else {
                    let _ = writeln!(s, "  \u{2192} {name}: installing via {}...", spec.program);
                }
            }
            PkgAction::Skipped => {
                let _ = writeln!(s, "  ~ {name}: skipped (--skip list)");
            }
            PkgAction::NotApplicable => {
                let _ = writeln!(s, "  ~ {name}: not applicable on this platform");
            }
            PkgAction::PlatformUnsupported => {
                let _ = writeln!(s, "  ! {name}: platform not recognised");
            }
        }
    }

    s
}

// ---------------------------------------------------------------------------
// Execution
// ---------------------------------------------------------------------------

/// Result for one package after execution.
#[derive(Debug)]
pub enum PkgResult {
    AlreadyInstalled,
    Installed,
    Failed(String),
    Skipped,
    NotApplicable,
    PlatformUnsupported,
}

/// Execute the install plan.  In dry-run mode the caller skips calling this.
pub(crate) fn execute_plan(plan: &PackageInstallPlan) -> Vec<(&Package, PkgResult)> {
    let mut results = Vec::new();

    for item in &plan.items {
        let result = match &item.action {
            PkgAction::AlreadyInstalled => PkgResult::AlreadyInstalled,
            PkgAction::Skipped => PkgResult::Skipped,
            PkgAction::NotApplicable => PkgResult::NotApplicable,
            PkgAction::PlatformUnsupported => PkgResult::PlatformUnsupported,
            PkgAction::WillInstall(spec) => {
                let name = item.pkg.display_name();
                println!("  \u{2192} {name}: installing via {}...", spec.program);
                match run_install(spec) {
                    Ok(()) => {
                        println!("    \u{2713} {name}: installed");
                        PkgResult::Installed
                    }
                    Err(e) => {
                        eprintln!("    error: {e}");
                        PkgResult::Failed(e.to_string())
                    }
                }
            }
        };
        results.push((&item.pkg, result));
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

// ---------------------------------------------------------------------------
// Summary
// ---------------------------------------------------------------------------

/// Print a summary table of the install results.
pub(crate) fn print_summary(results: &[(&Package, PkgResult)]) {
    let installed: Vec<_> = results
        .iter()
        .filter(|(_, r)| matches!(r, PkgResult::Installed))
        .map(|(p, _)| p.display_name())
        .collect();
    let failed: Vec<_> = results
        .iter()
        .filter(|(_, r)| matches!(r, PkgResult::Failed(_)))
        .map(|(p, _)| p.display_name())
        .collect();
    let already: usize = results
        .iter()
        .filter(|(_, r)| matches!(r, PkgResult::AlreadyInstalled))
        .count();

    println!();
    println!("Summary:");
    if installed.is_empty() && failed.is_empty() {
        if already > 0 {
            println!("  All requested packages already installed.");
        } else {
            println!("  Nothing to install.");
        }
    } else {
        if !installed.is_empty() {
            println!("  Installed:  {}", installed.join(", "));
        }
        if !failed.is_empty() {
            println!("  Failed:     {}", failed.join(", "));
        }
    }
}

// ---------------------------------------------------------------------------
// CLI args
// ---------------------------------------------------------------------------

/// Arguments for `neon setup install-packages`.
#[derive(Args, Debug)]
pub struct InstallPackagesArgs {
    /// Print what would run without executing anything.
    #[arg(long)]
    pub dry_run: bool,

    /// Skip specific packages by name (comma-separated).
    /// Valid names: zsh, oh-my-posh, oh-my-zsh, posh-git, lazygit, fzf, bat,
    /// zoxide, eza, git-delta, ripgrep, fd
    #[arg(long, value_delimiter = ',')]
    pub skip: Vec<String>,
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Entry point for `neon setup install-packages`.
pub fn run(args: &InstallPackagesArgs) -> Result<()> {
    let platform = current_platform();

    // Normalise skip list to lowercase for case-insensitive matching.
    let skip: Vec<String> = args.skip.iter().map(|s| s.to_lowercase()).collect();

    // Build the plan.
    let plan = build_plan(&skip, platform, is_installed);

    // Print the plan.
    print!("{}", format_plan(&plan, args.dry_run));

    if args.dry_run {
        return Ok(());
    }

    // Execute.
    let results = execute_plan(&plan);
    print_summary(&results);

    // Fail the command if any install failed.
    let any_failed = results
        .iter()
        .any(|(_, r)| matches!(r, PkgResult::Failed(_)));
    if any_failed {
        anyhow::bail!("one or more packages failed to install");
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn nothing_installed(_pkg: Package, _platform: Platform) -> bool {
        false
    }

    fn everything_installed(_pkg: Package, _platform: Platform) -> bool {
        true
    }

    // --- Probe name correctness ---

    #[test]
    fn package_names_are_correct() {
        assert_eq!(Package::Bat.name(), "bat");
        assert_eq!(Package::Eza.name(), "eza");
        assert_eq!(Package::OhMyPosh.name(), "oh-my-posh");
        assert_eq!(Package::OhMyZsh.name(), "oh-my-zsh");
        assert_eq!(Package::PoshGit.name(), "posh-git");
        assert_eq!(Package::GitDelta.name(), "git-delta");
        assert_eq!(Package::Ripgrep.name(), "ripgrep");
        assert_eq!(Package::Fd.name(), "fd");
        assert_eq!(Package::Lazygit.name(), "lazygit");
        assert_eq!(Package::Fzf.name(), "fzf");
        assert_eq!(Package::Zoxide.name(), "zoxide");
        assert_eq!(Package::Zsh.name(), "zsh");
    }

    // --- Platform routing ---

    #[test]
    fn windows_plan_uses_winget_for_most_tools() {
        let plan = build_plan(&[], Platform::Windows, nothing_installed);
        let output = format_plan(&plan, true);

        assert!(output.contains("winget"), "Windows plan should use winget");
        assert!(
            output.contains("powershell"),
            "Windows plan should use powershell for posh-git"
        );
    }

    #[test]
    fn windows_zsh_and_ohmyzsh_are_not_applicable() {
        let plan = build_plan(&[], Platform::Windows, nothing_installed);
        let output = format_plan(&plan, true);

        assert!(
            output.contains("not applicable"),
            "zsh/oh-my-zsh should be not applicable on Windows; got:\n{output}"
        );
    }

    #[test]
    fn linux_plan_uses_apt_and_curl() {
        let plan = build_plan(&[], Platform::Linux, nothing_installed);
        let output = format_plan(&plan, true);

        assert!(
            output.contains("apt-get") || output.contains("curl"),
            "Linux plan should mention apt-get or curl; got:\n{output}"
        );
    }

    #[test]
    fn linux_posh_git_is_not_applicable() {
        let plan = build_plan(&[], Platform::Linux, nothing_installed);
        let posh_item = plan
            .items
            .iter()
            .find(|i| i.pkg == Package::PoshGit)
            .expect("PoshGit in plan");
        assert_eq!(
            posh_item.action,
            PkgAction::NotApplicable,
            "posh-git should be NotApplicable on Linux"
        );
    }

    #[test]
    fn macos_plan_uses_brew() {
        let plan = build_plan(&[], Platform::MacOs, nothing_installed);
        let output = format_plan(&plan, true);

        assert!(output.contains("brew"), "macOS plan should use brew");
    }

    #[test]
    fn macos_posh_git_is_not_applicable() {
        let plan = build_plan(&[], Platform::MacOs, nothing_installed);
        let posh_item = plan
            .items
            .iter()
            .find(|i| i.pkg == Package::PoshGit)
            .expect("PoshGit in plan");
        assert_eq!(
            posh_item.action,
            PkgAction::NotApplicable,
            "posh-git should be NotApplicable on macOS"
        );
    }

    // --- Skip list ---

    #[test]
    fn skip_list_skips_named_packages() {
        let skip = vec!["bat".to_string(), "fzf".to_string()];
        let plan = build_plan(&skip, Platform::Windows, nothing_installed);

        let bat_item = plan
            .items
            .iter()
            .find(|i| i.pkg == Package::Bat)
            .expect("Bat in plan");
        let fzf_item = plan
            .items
            .iter()
            .find(|i| i.pkg == Package::Fzf)
            .expect("Fzf in plan");
        let rg_item = plan
            .items
            .iter()
            .find(|i| i.pkg == Package::Ripgrep)
            .expect("Ripgrep in plan");

        assert_eq!(bat_item.action, PkgAction::Skipped, "bat should be skipped");
        assert_eq!(fzf_item.action, PkgAction::Skipped, "fzf should be skipped");
        assert!(
            matches!(rg_item.action, PkgAction::WillInstall(_)),
            "ripgrep should be scheduled for install"
        );
    }

    #[test]
    fn unknown_skip_name_is_ignored() {
        let skip = vec!["nonexistent-tool".to_string()];
        let plan = build_plan(&skip, Platform::Linux, nothing_installed);
        let zsh_item = plan
            .items
            .iter()
            .find(|i| i.pkg == Package::Zsh)
            .expect("Zsh in plan");
        assert!(
            matches!(zsh_item.action, PkgAction::WillInstall(_)),
            "zsh should still be scheduled for install despite bogus skip entry"
        );
    }

    // --- Dry-run output ---

    #[test]
    fn dry_run_mentions_dry_run_and_would_run() {
        let plan = build_plan(&[], Platform::Linux, nothing_installed);
        let output = format_plan(&plan, true);

        assert!(
            output.contains("dry run"),
            "dry-run output should say 'dry run'"
        );
        assert!(
            output.contains("would run"),
            "dry-run output should say 'would run'"
        );
    }

    #[test]
    fn dry_run_shows_commands_for_linux_packages() {
        let plan = build_plan(&[], Platform::Linux, nothing_installed);
        let output = format_plan(&plan, true);

        assert!(
            output.contains("apt-get"),
            "Linux dry-run should mention apt-get; got:\n{output}"
        );
        assert!(
            output.contains("curl"),
            "Linux dry-run should mention curl for oh-my-posh; got:\n{output}"
        );
    }

    // --- Already installed ---

    #[test]
    fn already_installed_packages_show_checkmark() {
        let plan = build_plan(&[], Platform::Windows, everything_installed);
        let output = format_plan(&plan, false);

        assert!(
            output.contains("already installed"),
            "all-installed plan should say 'already installed'; got:\n{output}"
        );
    }

    // --- Windows specific commands ---

    #[test]
    fn windows_winget_ids_are_correct() {
        let bat_spec = install_spec(Package::Bat, Platform::Windows).expect("bat has Windows spec");
        assert!(
            bat_spec.display().contains("sharkdp.bat"),
            "bat winget ID should be sharkdp.bat; got: {}",
            bat_spec.display()
        );

        let rg_spec =
            install_spec(Package::Ripgrep, Platform::Windows).expect("ripgrep has Windows spec");
        assert!(
            rg_spec.display().contains("BurntSushi.ripgrep.MSVC"),
            "ripgrep winget ID should be BurntSushi.ripgrep.MSVC; got: {}",
            rg_spec.display()
        );

        let delta_spec =
            install_spec(Package::GitDelta, Platform::Windows).expect("git-delta has Windows spec");
        assert!(
            delta_spec.display().contains("dandavison.delta"),
            "git-delta winget ID should be dandavison.delta; got: {}",
            delta_spec.display()
        );
    }
}
