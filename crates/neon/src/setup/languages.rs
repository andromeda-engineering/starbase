/// `neon setup install-languages` — idempotently install language toolchains.
///
/// Languages: nvm+node (LTS), python3, rustup, go.
/// Each is probed first; already-present tools print ✓ and are skipped.
use anyhow::Result;

use super::common::on_path;

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

// --- Language enum ---

/// A language toolchain that `neon setup install-languages` can install.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    Node,
    Python,
    Rust,
    Go,
}

impl Language {
    /// All four languages, in canonical installation order.
    pub fn all() -> &'static [Language] {
        &[
            Language::Node,
            Language::Python,
            Language::Rust,
            Language::Go,
        ]
    }

    /// Short name used in `--skip` list and output lines.
    pub fn name(self) -> &'static str {
        match self {
            Language::Node => "node",
            Language::Python => "python",
            Language::Rust => "rust",
            Language::Go => "go",
        }
    }
}

// --- Install spec ---

/// A single install command (program + args) for a given step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LangInstallSpec {
    pub program: String,
    pub args: Vec<String>,
}

impl LangInstallSpec {
    fn new(program: impl Into<String>, args: &[&str]) -> Self {
        LangInstallSpec {
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

/// A sequence of install steps for a language (node needs nvm first, then two
/// nvm sub-commands; all others are single-step).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LangInstallPlan {
    pub steps: Vec<LangInstallSpec>,
}

/// Resolve the install sequence for `language` on the given platform.
/// Returns `None` on `Unknown` platform.
fn install_steps(language: Language, platform: Platform) -> Option<LangInstallPlan> {
    match platform {
        Platform::Windows => Some(match language {
            // nvm for Windows: one winget invocation, then nvm install/use as separate steps.
            Language::Node => LangInstallPlan {
                steps: vec![
                    LangInstallSpec::new(
                        "winget",
                        &[
                            "install",
                            "--id",
                            "CoreyButler.NVMforWindows",
                            "-e",
                            "--accept-source-agreements",
                            "--accept-package-agreements",
                        ],
                    ),
                    LangInstallSpec::new("nvm", &["install", "lts"]),
                    LangInstallSpec::new("nvm", &["use", "lts"]),
                ],
            },
            Language::Python => LangInstallPlan {
                steps: vec![LangInstallSpec::new(
                    "winget",
                    &[
                        "install",
                        "--id",
                        "Python.Python.3",
                        "--source",
                        "winget",
                        "--accept-source-agreements",
                        "--accept-package-agreements",
                    ],
                )],
            },
            Language::Rust => LangInstallPlan {
                steps: vec![LangInstallSpec::new(
                    "winget",
                    &[
                        "install",
                        "--id",
                        "Rustlang.Rustup",
                        "-e",
                        "--accept-source-agreements",
                        "--accept-package-agreements",
                    ],
                )],
            },
            Language::Go => LangInstallPlan {
                steps: vec![LangInstallSpec::new(
                    "winget",
                    &[
                        "install",
                        "--id",
                        "GoLang.Go",
                        "-e",
                        "--accept-source-agreements",
                        "--accept-package-agreements",
                    ],
                )],
            },
        }),
        Platform::Linux => Some(match language {
            Language::Node => LangInstallPlan {
                steps: vec![
                    LangInstallSpec::new(
                        "bash",
                        &[
                            "-c",
                            "curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.40.3/install.sh | bash",
                        ],
                    ),
                    LangInstallSpec::new("bash", &["-c", "source ~/.nvm/nvm.sh && nvm install --lts"]),
                ],
            },
            Language::Python => LangInstallPlan {
                steps: vec![LangInstallSpec::new(
                    "sudo",
                    &["apt-get", "install", "-y", "python3", "python3-pip"],
                )],
            },
            Language::Rust => LangInstallPlan {
                steps: vec![LangInstallSpec::new(
                    "bash",
                    &[
                        "-c",
                        "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y",
                    ],
                )],
            },
            Language::Go => LangInstallPlan {
                steps: vec![LangInstallSpec::new(
                    "sudo",
                    &["apt-get", "install", "-y", "golang-go"],
                )],
            },
        }),
        Platform::MacOs => Some(match language {
            Language::Node => LangInstallPlan {
                steps: vec![
                    LangInstallSpec::new(
                        "bash",
                        &[
                            "-c",
                            "curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.40.3/install.sh | bash",
                        ],
                    ),
                    LangInstallSpec::new("bash", &["-c", "source ~/.nvm/nvm.sh && nvm install --lts"]),
                ],
            },
            Language::Python => LangInstallPlan {
                steps: vec![LangInstallSpec::new("brew", &["install", "python3"])],
            },
            Language::Rust => LangInstallPlan {
                steps: vec![LangInstallSpec::new(
                    "bash",
                    &[
                        "-c",
                        "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y",
                    ],
                )],
            },
            Language::Go => LangInstallPlan {
                steps: vec![LangInstallSpec::new("brew", &["install", "go"])],
            },
        }),
        Platform::Unknown => None,
    }
}

// --- Probe (injectable for tests) ---

/// Returns `true` if `language` is already present on PATH.
///
/// The probe function is injectable so tests can pass a fake without real PATH
/// lookups.  The real probe is `default_probe`.
pub(crate) fn default_probe(language: Language, platform: Platform) -> bool {
    match language {
        Language::Node => on_path("node"),
        // Python is `python` on Windows, `python3` elsewhere.
        Language::Python => {
            if platform == Platform::Windows {
                on_path("python") || on_path("python3")
            } else {
                on_path("python3")
            }
        }
        Language::Rust => on_path("rustup"),
        Language::Go => on_path("go"),
    }
}

// --- Plan types ---

/// The resolved action for one language.
#[derive(Debug, PartialEq, Eq)]
pub enum LangAction {
    /// Language toolchain is already installed — no action needed.
    AlreadyInstalled,
    /// Language toolchain will be installed using the given steps.
    WillInstall(LangInstallPlan),
    /// Language is in the `--skip` list — no action needed.
    Skipped,
    /// Platform not yet supported for this language.
    PlatformUnsupported,
}

/// The plan for a single language.
#[derive(Debug)]
pub struct LangPlan {
    pub language: Language,
    pub action: LangAction,
}

/// The full plan for all languages.
#[derive(Debug)]
pub struct LanguageInstallPlan {
    pub items: Vec<LangPlan>,
}

// --- Plan derivation (pure, no I/O) ---

/// Build a plan without executing anything.
///
/// `skip`  — languages the user asked to skip (via `--skip`).
/// `platform` — the resolved platform.
/// `probe` — injectable probe: returns true if language is already installed.
pub(crate) fn build_plan(
    skip: &[String],
    platform: Platform,
    probe: impl Fn(Language, Platform) -> bool,
) -> LanguageInstallPlan {
    let items = Language::all()
        .iter()
        .map(|&lang| {
            if skip.iter().any(|s| s == lang.name()) {
                return LangPlan {
                    language: lang,
                    action: LangAction::Skipped,
                };
            }
            if probe(lang, platform) {
                return LangPlan {
                    language: lang,
                    action: LangAction::AlreadyInstalled,
                };
            }
            let action = match install_steps(lang, platform) {
                Some(plan) => LangAction::WillInstall(plan),
                None => LangAction::PlatformUnsupported,
            };
            LangPlan {
                language: lang,
                action,
            }
        })
        .collect();

    LanguageInstallPlan { items }
}

// --- Formatting (pure) ---

/// Format the plan as a human-readable string.  Pure — suitable for testing.
pub fn format_plan(plan: &LanguageInstallPlan, dry_run: bool) -> String {
    use std::fmt::Write;
    let mut s = String::new();

    if dry_run {
        let _ = writeln!(
            s,
            "Installing language toolchains... (dry run \u{2014} no changes will be made)"
        );
    } else {
        let _ = writeln!(s, "Installing language toolchains...");
    }

    for item in &plan.items {
        let name = item.language.name();
        match &item.action {
            LangAction::AlreadyInstalled => {
                let _ = writeln!(s, "  \u{2713} {name} already installed");
            }
            LangAction::WillInstall(lang_plan) => {
                if dry_run {
                    for step in &lang_plan.steps {
                        let _ = writeln!(s, "  \u{2192} {name}: would run: {}", step.display());
                    }
                } else {
                    let _ = writeln!(s, "  \u{2192} installing {name}...");
                }
            }
            LangAction::Skipped => {
                let _ = writeln!(s, "  ~ {name}: skipped (--skip list)");
            }
            LangAction::PlatformUnsupported => {
                let _ = writeln!(s, "  ! {name}: not yet supported on this platform");
            }
        }
    }

    s
}

// --- Execution ---

/// Result for one language after execution.
#[derive(Debug)]
pub enum LangResult {
    AlreadyInstalled,
    Installed,
    Failed(String),
    Skipped,
    PlatformUnsupported,
}

/// Execute the install plan (not called in dry-run mode).
///
/// Note: multi-step installs (e.g. node via nvm) run each step in sequence and
/// stop on the first failure.  After installing nvm, the subsequent `nvm`
/// sub-commands require a new shell session to have nvm on PATH; callers should
/// print a reminder to restart the shell after a real (non-dry-run) install.
fn execute_plan(plan: &LanguageInstallPlan) -> Vec<(&Language, LangResult)> {
    let mut results = Vec::new();

    for item in &plan.items {
        let result = match &item.action {
            LangAction::AlreadyInstalled => LangResult::AlreadyInstalled,
            LangAction::Skipped => LangResult::Skipped,
            LangAction::PlatformUnsupported => LangResult::PlatformUnsupported,
            LangAction::WillInstall(lang_plan) => {
                let name = item.language.name();
                println!("  \u{2192} installing {name}...");
                let mut failed: Option<String> = None;
                for step in &lang_plan.steps {
                    match run_step(step) {
                        Ok(()) => {}
                        Err(e) => {
                            eprintln!("    error: {e}");
                            failed = Some(e.to_string());
                            break;
                        }
                    }
                }
                match failed {
                    None => {
                        println!("  \u{2713} {name} installed");
                        LangResult::Installed
                    }
                    Some(msg) => LangResult::Failed(msg),
                }
            }
        };
        results.push((&item.language, result));
    }

    results
}

fn run_step(spec: &LangInstallSpec) -> Result<()> {
    let status = std::process::Command::new(&spec.program)
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

fn print_summary(results: &[(&Language, LangResult)]) {
    let installed: Vec<_> = results
        .iter()
        .filter(|(_, r)| matches!(r, LangResult::Installed))
        .map(|(l, _)| l.name())
        .collect();
    let failed: Vec<_> = results
        .iter()
        .filter(|(_, r)| matches!(r, LangResult::Failed(_)))
        .map(|(l, _)| l.name())
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

/// Arguments for `neon setup install-languages`.
#[derive(clap::Args, Debug)]
pub struct InstallLanguagesArgs {
    /// Print what would be done without doing it
    #[arg(long)]
    pub dry_run: bool,
    /// Comma-separated list of languages to skip (node, python, rust, go)
    #[arg(long, value_delimiter = ',')]
    pub skip: Vec<String>,
}

// --- Public entry point ---

pub fn run(args: &InstallLanguagesArgs) -> Result<()> {
    let platform = current_platform();

    // Build the plan using real PATH probes.
    let plan = build_plan(&args.skip, platform, default_probe);

    // Print the plan.
    print!("{}", format_plan(&plan, args.dry_run));

    if args.dry_run {
        return Ok(());
    }

    // Execute.
    let results = execute_plan(&plan);
    print_summary(&results);

    // Print shell-restart hint if node was installed (nvm requires a new session).
    let node_installed = results
        .iter()
        .any(|(l, r)| l.name() == "node" && matches!(r, LangResult::Installed));
    if node_installed {
        println!();
        println!(
            "  Note: nvm was installed. Restart your shell (or open a new terminal) before \
             running `nvm` or `node` so the PATH update takes effect."
        );
    }

    // Fail the command if any install failed.
    let any_failed = results
        .iter()
        .any(|(_, r)| matches!(r, LangResult::Failed(_)));
    if any_failed {
        anyhow::bail!("one or more language toolchains failed to install");
    }

    Ok(())
}

// --- Tests ---

#[cfg(test)]
mod tests {
    use super::*;

    /// Probe that reports nothing installed.
    fn nothing_installed(_lang: Language, _platform: Platform) -> bool {
        false
    }

    /// Probe that reports everything installed.
    fn everything_installed(_lang: Language, _platform: Platform) -> bool {
        true
    }

    // --- skip list ---

    #[test]
    fn skip_list_excludes_named_languages() {
        let skip = vec!["python".to_string(), "go".to_string()];
        let plan = build_plan(&skip, Platform::Windows, nothing_installed);

        for item in &plan.items {
            match item.language {
                Language::Python | Language::Go => {
                    assert_eq!(
                        item.action,
                        LangAction::Skipped,
                        "{} should be skipped",
                        item.language.name()
                    );
                }
                Language::Node | Language::Rust => {
                    assert!(
                        !matches!(item.action, LangAction::Skipped),
                        "{} should NOT be skipped",
                        item.language.name()
                    );
                }
            }
        }
    }

    #[test]
    fn skip_empty_list_installs_all() {
        let plan = build_plan(&[], Platform::Windows, nothing_installed);
        for item in &plan.items {
            assert!(
                !matches!(item.action, LangAction::Skipped),
                "{} should not be skipped with empty skip list",
                item.language.name()
            );
        }
    }

    // --- dry-run output ---

    #[test]
    fn dry_run_windows_shows_all_four_commands() {
        let plan = build_plan(&[], Platform::Windows, nothing_installed);
        let output = format_plan(&plan, true);

        assert!(output.contains("dry run"), "should mention dry run");
        for lang in Language::all() {
            assert!(
                output.contains(lang.name()),
                "output should mention language '{}'",
                lang.name()
            );
        }
        // All four should show winget on Windows.
        assert!(
            output.contains("winget"),
            "Windows plan should mention winget; got:\n{output}"
        );
        // No language should be marked skipped.
        assert!(!output.contains("skipped"), "nothing should be skipped");
    }

    #[test]
    fn dry_run_linux_shows_bash_and_apt() {
        let plan = build_plan(&[], Platform::Linux, nothing_installed);
        let output = format_plan(&plan, true);

        // node and rust use bash; python and go use apt-get.
        assert!(
            output.contains("bash") || output.contains("apt-get"),
            "Linux plan should mention bash or apt-get; got:\n{output}"
        );
    }

    #[test]
    fn dry_run_macos_shows_brew_and_bash() {
        let plan = build_plan(&[], Platform::MacOs, nothing_installed);
        let output = format_plan(&plan, true);

        // python and go use brew; node and rust use bash.
        assert!(
            output.contains("brew") || output.contains("bash"),
            "macOS plan should mention brew or bash; got:\n{output}"
        );
    }

    // --- already-present tools are skipped ---

    #[test]
    fn already_installed_shows_checkmark_no_colon() {
        let plan = build_plan(&[], Platform::Windows, everything_installed);
        let output = format_plan(&plan, false);

        for lang in Language::all() {
            let expected = format!("\u{2713} {} already installed", lang.name());
            assert!(
                output.contains(&expected),
                "expected '{}' in output; got:\n{output}",
                expected
            );
        }
        // Must not contain install arrows.
        assert!(
            !output.contains('\u{2192}'),
            "no install arrows expected when all already installed; got:\n{output}"
        );
    }

    // --- node has multiple steps ---

    #[test]
    fn node_install_has_multiple_steps_on_windows() {
        let plan = build_plan(&[], Platform::Windows, nothing_installed);
        let node_plan = plan
            .items
            .iter()
            .find(|i| i.language == Language::Node)
            .unwrap();
        if let LangAction::WillInstall(lp) = &node_plan.action {
            assert!(
                lp.steps.len() > 1,
                "node on Windows should have multiple steps (nvm install + use); got {}",
                lp.steps.len()
            );
            // First step should be winget for nvm.
            assert_eq!(lp.steps[0].program, "winget");
            assert!(
                lp.steps[0].args.iter().any(|a| a.contains("NVMforWindows")),
                "first step should install NVMforWindows"
            );
        } else {
            panic!("expected WillInstall for node on Windows");
        }
    }

    #[test]
    fn node_install_has_multiple_steps_on_linux() {
        let plan = build_plan(&[], Platform::Linux, nothing_installed);
        let node_plan = plan
            .items
            .iter()
            .find(|i| i.language == Language::Node)
            .unwrap();
        if let LangAction::WillInstall(lp) = &node_plan.action {
            assert!(
                lp.steps.len() > 1,
                "node on Linux should have multiple steps; got {}",
                lp.steps.len()
            );
        } else {
            panic!("expected WillInstall for node on Linux");
        }
    }

    // --- unknown platform ---

    #[test]
    fn unknown_platform_marks_unsupported() {
        let plan = build_plan(&[], Platform::Unknown, nothing_installed);
        for item in &plan.items {
            assert_eq!(
                item.action,
                LangAction::PlatformUnsupported,
                "{} should be unsupported on Unknown platform",
                item.language.name()
            );
        }
    }

    // --- LangInstallSpec display ---

    #[test]
    fn lang_install_spec_display_contains_program_and_args() {
        let spec = LangInstallSpec::new("winget", &["install", "--id", "GoLang.Go"]);
        let display = spec.display();
        assert!(display.starts_with("winget"));
        assert!(display.contains("GoLang.Go"));
    }

    #[test]
    fn lang_install_spec_display_no_args() {
        let spec = LangInstallSpec::new("nvm", &[]);
        assert_eq!(spec.display(), "nvm");
    }

    // --- language names ---

    #[test]
    fn language_names_are_canonical() {
        assert_eq!(Language::Node.name(), "node");
        assert_eq!(Language::Python.name(), "python");
        assert_eq!(Language::Rust.name(), "rust");
        assert_eq!(Language::Go.name(), "go");
    }
}
