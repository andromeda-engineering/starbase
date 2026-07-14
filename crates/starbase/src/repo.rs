use anyhow::Result;
use clap::{Args, ValueEnum};
use std::path::PathBuf;

// --- Profile inputs (§7 of docs/architecture/repo-setup-automation.md) ---

/// Repository topology: single-maintainer vs multi-person team.
///
/// Affects GitHub branch-protection settings applied by `starbase repo harden`
/// (solo → admin bypass; team → strict approvals).
#[derive(ValueEnum, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Topology {
    /// Single-maintainer repo — admin bypass on branch protection.
    #[default]
    Solo,
    /// Multi-person team — strict approval requirements.
    Team,
}

/// Repository visibility on GitHub.
///
/// Affects which security features are available at harden time.
#[derive(ValueEnum, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Visibility {
    /// Public repo — free secret scanning and push protection.
    #[default]
    Public,
    /// Private repo — paid security features path.
    Private,
}

/// npm publish target for this repo.
///
/// Gates the release workflow and provenance configuration.
#[derive(ValueEnum, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Publish {
    /// No npm publishing — skip release workflow.
    #[default]
    None,
    /// Wire up Changesets but defer npm publish; a human triggers the release.
    Changesets,
    /// Full npm publish on merge (provenance + access configured).
    NpmNow,
}

/// Merge strategy profile.
///
/// Controls which merge methods GitHub enables on the repo.
#[derive(ValueEnum, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Merge {
    /// Squash-only — standard for libraries / OSS packages.
    #[default]
    Library,
    /// Squash + rebase — suits application repos that want linear history options.
    App,
}

/// Language stack present in the repository.
///
/// Determines which CI jobs, Cargo workspace, and Turborepo config are generated.
#[derive(ValueEnum, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Languages {
    /// TypeScript/JS only — no Rust toolchain, no Cargo workspace.
    Ts,
    /// Rust only — no pnpm/Turborepo, no TypeScript CI.
    Rust,
    /// Both TypeScript and Rust (the canonical Starbase profile).
    #[default]
    Both,
    /// Bare repo — community/docs/.github files only; no language workspace.
    Bare,
}

/// License for the repository.
///
/// Controls which LICENSE file is emitted by `starbase repo init`, or omits it
/// entirely for proprietary repositories.
#[derive(ValueEnum, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum License {
    /// MIT License (permissive).
    #[default]
    Mit,
    /// Apache License 2.0 (permissive with patent grant).
    Apache2,
    /// GNU General Public License v3.0 (copyleft).
    Gpl3,
    /// BSD 3-Clause License (permissive).
    Bsd3Clause,
    /// Mozilla Public License 2.0 (weak copyleft).
    Mpl2,
    /// The Unlicense (public domain dedication).
    Unlicense,
    /// Proprietary — no LICENSE file emitted.
    Proprietary,
}

// --- Canonical profile ---

/// The full set of configurable inputs that drive scaffold and harden behaviour.
///
/// The `Default` impl encodes the canonical OSS + solo + both-languages +
/// defer-publish profile used when no flags are supplied.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct RepoProfile {
    pub topology: Topology,
    pub visibility: Visibility,
    pub publish: Publish,
    pub merge: Merge,
    pub languages: Languages,
    pub license: License,
}

// --- CLI args struct ---

/// Arguments for `starbase repo init`.
#[derive(Args, Debug)]
pub struct InitArgs {
    /// Target directory for the new repository (default: current directory).
    ///
    /// In this dry-run slice the path is echoed in the plan but no files are
    /// written and no directory is created.
    #[arg(value_name = "PATH")]
    pub path: Option<PathBuf>,

    /// Repository topology (solo = single maintainer, team = multi-person).
    #[arg(long, value_enum, default_value = "solo")]
    pub topology: Topology,

    /// Repository visibility on GitHub.
    #[arg(long, value_enum, default_value = "public")]
    pub visibility: Visibility,

    /// npm publish strategy.
    #[arg(long, value_enum, default_value = "none")]
    pub publish: Publish,

    /// Merge strategy for the repository.
    #[arg(long, value_enum, default_value = "library")]
    pub merge: Merge,

    /// Language stack present in the repository.
    #[arg(long, value_enum, default_value = "both")]
    pub languages: Languages,

    /// License for the repository (use "proprietary" to omit the LICENSE file).
    #[arg(long, value_enum, default_value = "mit")]
    pub license: License,
}

impl InitArgs {
    /// Build a `RepoProfile` from the parsed CLI arguments.
    fn into_profile(self) -> RepoProfile {
        RepoProfile {
            topology: self.topology,
            visibility: self.visibility,
            publish: self.publish,
            merge: self.merge,
            languages: self.languages,
            license: self.license,
        }
    }
}

// --- Plan derivation (pure, no I/O) ---

/// A single item in the scaffold plan.
#[derive(Debug, PartialEq, Eq)]
pub struct PlanItem {
    /// Short label for what would be created.
    pub label: String,
}

impl PlanItem {
    fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
        }
    }
}

/// Derive the list of files / artifacts that `starbase repo init` *would* generate for
/// the given profile.  Pure function — no filesystem access, no I/O.
pub fn plan(profile: &RepoProfile) -> Vec<PlanItem> {
    let license_label: Option<&str> = match profile.license {
        License::Mit => Some("LICENSE (MIT)"),
        License::Apache2 => Some("LICENSE (Apache-2.0)"),
        License::Gpl3 => Some("LICENSE (GPL-3.0)"),
        License::Bsd3Clause => Some("LICENSE (BSD-3-Clause)"),
        License::Mpl2 => Some("LICENSE (MPL-2.0)"),
        License::Unlicense => Some("LICENSE (Unlicense)"),
        License::Proprietary => None,
    };

    // Start with language-independent community & docs files; extend conditionally.
    let mut items: Vec<PlanItem> = vec![PlanItem::new("README.md")];
    if let Some(label) = license_label {
        items.push(PlanItem::new(label));
    }
    items.extend([
        PlanItem::new("CONTRIBUTING.md"),
        PlanItem::new("CODE_OF_CONDUCT.md (Contributor Covenant v2.1)"),
        PlanItem::new("SECURITY.md"),
        PlanItem::new("CHANGELOG.md"),
        // GitHub community files
        PlanItem::new(".github/PULL_REQUEST_TEMPLATE.md"),
        PlanItem::new(".github/ISSUE_TEMPLATE/bug_report.yml"),
        PlanItem::new(".github/ISSUE_TEMPLATE/feature_request.yml"),
        PlanItem::new(".github/ISSUE_TEMPLATE/documentation.yml"),
        PlanItem::new(".github/ISSUE_TEMPLATE/config.yml"),
    ]);

    // --- Dependabot and CodeRabbit (always) ---
    match profile.languages {
        Languages::Ts => {
            items.push(PlanItem::new(".github/dependabot.yml (npm ecosystem)"));
        }
        Languages::Rust => {
            items.push(PlanItem::new(".github/dependabot.yml (cargo ecosystem)"));
        }
        Languages::Both => {
            items.push(PlanItem::new(
                ".github/dependabot.yml (npm + cargo ecosystems)",
            ));
        }
        Languages::Bare => {
            items.push(PlanItem::new(
                ".github/dependabot.yml (github-actions only)",
            ));
        }
    }
    items.push(PlanItem::new(".coderabbit.yaml"));

    // --- TypeScript / JS workspace files ---
    if matches!(profile.languages, Languages::Ts | Languages::Both) {
        items.push(PlanItem::new("package.json (type=module, workspace root)"));
        items.push(PlanItem::new("pnpm-workspace.yaml"));
        items.push(PlanItem::new("turbo.json"));
        items.push(PlanItem::new("tsconfig.base.json"));
        items.push(PlanItem::new("eslint.config.js"));
        items.push(PlanItem::new(".github/workflows/typescript.yml"));
    }

    // --- Rust / Cargo workspace files ---
    if matches!(profile.languages, Languages::Rust | Languages::Both) {
        items.push(PlanItem::new("Cargo.toml (workspace)"));
        items.push(PlanItem::new("rust-toolchain.toml"));
        items.push(PlanItem::new("clippy.toml"));
        items.push(PlanItem::new(".github/workflows/rust.yml"));
    }

    // --- Publish: changesets or npm-now ---
    match profile.publish {
        Publish::None => {}
        Publish::Changesets => {
            items.push(PlanItem::new(".changeset/config.json"));
            items.push(PlanItem::new(
                ".github/workflows/release.yml (changesets — human-triggered)",
            ));
        }
        Publish::NpmNow => {
            items.push(PlanItem::new(".changeset/config.json"));
            items.push(PlanItem::new(
                ".github/workflows/release.yml (npm publish on merge, provenance enabled)",
            ));
        }
    }

    items
}

/// Format the scaffold plan as a human-readable string, including the profile
/// summary.  Pure function — suitable for unit-testing without capturing stdout.
pub fn format_plan(profile: &RepoProfile, path: Option<&PathBuf>) -> String {
    use std::fmt::Write;
    let mut s = String::new();

    let target = path
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| ".".to_string());

    let _ = writeln!(s, "=== starbase repo init — DRY RUN ===");
    let _ = writeln!(s, "  target:     {target}");
    let _ = writeln!(s);
    let _ = writeln!(s, "=== Profile ===");
    let _ = writeln!(s, "  topology:   {:?}", profile.topology);
    let _ = writeln!(s, "  visibility: {:?}", profile.visibility);
    let _ = writeln!(s, "  publish:    {:?}", profile.publish);
    let _ = writeln!(s, "  merge:      {:?}", profile.merge);
    let _ = writeln!(s, "  languages:  {:?}", profile.languages);
    let _ = writeln!(s, "  license:    {:?}", profile.license);
    let _ = writeln!(s);
    let _ = writeln!(s, "=== Would generate ===");

    for item in plan(profile) {
        let _ = writeln!(s, "  would create: {}", item.label);
    }

    let _ = writeln!(s);
    let _ = writeln!(s, "No files were written (dry-run).");

    s
}

// --- Cross-field validation ---

/// Validate constraints that span more than one profile field.
///
/// `--publish changesets|npm-now` implies npm release artifacts (`.changeset`,
/// a release workflow), which only make sense for a JS/TS workspace. A Rust-only
/// repo has no npm package to publish, so reject the combination instead of
/// emitting a contradictory plan.
///
/// Similarly, a bare repo has no language workspace and therefore no npm package
/// to publish.
fn validate(profile: &RepoProfile) -> Result<()> {
    if matches!(profile.languages, Languages::Rust) && !matches!(profile.publish, Publish::None) {
        anyhow::bail!(
            "--publish changesets|npm-now requires --languages ts|both \
             (a Rust-only repo has no npm package to publish)"
        );
    }
    if matches!(profile.languages, Languages::Bare) && !matches!(profile.publish, Publish::None) {
        anyhow::bail!("--publish requires a language workspace; bare repos have no npm package");
    }
    Ok(())
}

// --- Public entry point ---

/// Entry point for `starbase repo init`.
///
/// Builds a `RepoProfile` from the parsed CLI arguments and prints a
/// human-readable scaffold plan to stdout.  No files are written and no
/// network or git calls are made in this slice.
pub fn init(args: InitArgs) -> Result<()> {
    let path = args.path.clone();
    let profile = args.into_profile();
    validate(&profile)?;
    print!("{}", format_plan(&profile, path.as_ref()));
    Ok(())
}

// --- Tests ---

#[cfg(test)]
mod tests {
    use super::*;

    fn plan_labels(profile: &RepoProfile) -> Vec<String> {
        plan(profile).into_iter().map(|i| i.label).collect()
    }

    #[test]
    fn default_profile_is_canonical_oss_solo_both() {
        let p = RepoProfile::default();
        assert_eq!(p.topology, Topology::Solo);
        assert_eq!(p.visibility, Visibility::Public);
        assert_eq!(p.publish, Publish::None);
        assert_eq!(p.merge, Merge::Library);
        assert_eq!(p.languages, Languages::Both);
        assert_eq!(p.license, License::Mit);
    }

    #[test]
    fn both_languages_includes_rust_and_ts_entries() {
        let profile = RepoProfile {
            languages: Languages::Both,
            ..RepoProfile::default()
        };
        let labels = plan_labels(&profile);
        assert!(
            labels.iter().any(|l| l.contains("Cargo.toml")),
            "Both profile should include Cargo.toml; got: {labels:?}"
        );
        assert!(
            labels.iter().any(|l| l.contains("rust.yml")),
            "Both profile should include rust.yml; got: {labels:?}"
        );
        assert!(
            labels.iter().any(|l| l.contains("package.json")),
            "Both profile should include package.json; got: {labels:?}"
        );
        assert!(
            labels.iter().any(|l| l.contains("turbo.json")),
            "Both profile should include turbo.json; got: {labels:?}"
        );
        assert!(
            labels.iter().any(|l| l.contains("typescript.yml")),
            "Both profile should include typescript.yml; got: {labels:?}"
        );
    }

    #[test]
    fn ts_only_omits_rust_entries() {
        let profile = RepoProfile {
            languages: Languages::Ts,
            ..RepoProfile::default()
        };
        let labels = plan_labels(&profile);
        assert!(
            !labels.iter().any(|l| l.contains("Cargo.toml")),
            "TS-only profile must omit Cargo.toml; got: {labels:?}"
        );
        assert!(
            !labels.iter().any(|l| l.contains("rust.yml")),
            "TS-only profile must omit rust.yml; got: {labels:?}"
        );
        assert!(
            !labels.iter().any(|l| l.contains("rust-toolchain")),
            "TS-only profile must omit rust-toolchain.toml; got: {labels:?}"
        );
        assert!(
            labels.iter().any(|l| l.contains("package.json")),
            "TS-only profile must include package.json; got: {labels:?}"
        );
        assert!(
            labels.iter().any(|l| l.contains("typescript.yml")),
            "TS-only profile must include typescript.yml; got: {labels:?}"
        );
    }

    #[test]
    fn rust_only_omits_ts_entries() {
        let profile = RepoProfile {
            languages: Languages::Rust,
            ..RepoProfile::default()
        };
        let labels = plan_labels(&profile);
        assert!(
            !labels.iter().any(|l| l.contains("package.json")),
            "Rust-only profile must omit package.json; got: {labels:?}"
        );
        assert!(
            !labels.iter().any(|l| l.contains("turbo.json")),
            "Rust-only profile must omit turbo.json; got: {labels:?}"
        );
        assert!(
            !labels.iter().any(|l| l.contains("typescript.yml")),
            "Rust-only profile must omit typescript.yml; got: {labels:?}"
        );
        assert!(
            !labels.iter().any(|l| l.contains("pnpm")),
            "Rust-only profile must omit pnpm entries; got: {labels:?}"
        );
        assert!(
            labels.iter().any(|l| l.contains("Cargo.toml")),
            "Rust-only profile must include Cargo.toml; got: {labels:?}"
        );
        assert!(
            labels.iter().any(|l| l.contains("rust.yml")),
            "Rust-only profile must include rust.yml; got: {labels:?}"
        );
    }

    #[test]
    fn publish_none_omits_release_workflow() {
        let profile = RepoProfile {
            publish: Publish::None,
            ..RepoProfile::default()
        };
        let labels = plan_labels(&profile);
        assert!(
            !labels.iter().any(|l| l.contains("release.yml")),
            "Publish=None must omit release.yml; got: {labels:?}"
        );
        assert!(
            !labels.iter().any(|l| l.contains(".changeset")),
            "Publish=None must omit .changeset; got: {labels:?}"
        );
    }

    #[test]
    fn publish_changesets_includes_release_workflow() {
        let profile = RepoProfile {
            publish: Publish::Changesets,
            ..RepoProfile::default()
        };
        let labels = plan_labels(&profile);
        assert!(
            labels.iter().any(|l| l.contains(".changeset/config.json")),
            "Changesets publish must include .changeset/config.json; got: {labels:?}"
        );
        assert!(
            labels.iter().any(|l| l.contains("release.yml")),
            "Changesets publish must include release.yml; got: {labels:?}"
        );
    }

    #[test]
    fn validate_rejects_rust_only_with_publish() {
        let profile = RepoProfile {
            languages: Languages::Rust,
            publish: Publish::NpmNow,
            ..RepoProfile::default()
        };
        assert!(
            validate(&profile).is_err(),
            "rust-only + publish should be rejected"
        );
    }

    #[test]
    fn validate_allows_rust_only_without_publish() {
        let profile = RepoProfile {
            languages: Languages::Rust,
            publish: Publish::None,
            ..RepoProfile::default()
        };
        assert!(validate(&profile).is_ok());
    }

    #[test]
    fn validate_allows_publish_with_ts_or_both() {
        for languages in [Languages::Ts, Languages::Both] {
            let profile = RepoProfile {
                languages,
                publish: Publish::NpmNow,
                ..RepoProfile::default()
            };
            assert!(
                validate(&profile).is_ok(),
                "publish should be allowed for {languages:?}"
            );
        }
    }

    #[test]
    fn format_plan_contains_profile_summary_and_items() {
        let profile = RepoProfile::default();
        let output = format_plan(&profile, None);
        assert!(output.contains("=== starbase repo init"));
        assert!(output.contains("=== Profile ==="));
        assert!(output.contains("=== Would generate ==="));
        assert!(output.contains("would create:"));
        assert!(output.contains("No files were written (dry-run)."));
        assert!(output.contains("target:     ."));
    }

    #[test]
    fn bare_omits_workspace_files() {
        let profile = RepoProfile {
            languages: Languages::Bare,
            ..RepoProfile::default()
        };
        let labels = plan_labels(&profile);
        assert!(
            !labels.iter().any(|l| l.contains("package.json")),
            "Bare profile must omit package.json; got: {labels:?}"
        );
        assert!(
            !labels.iter().any(|l| l.contains("Cargo.toml")),
            "Bare profile must omit Cargo.toml; got: {labels:?}"
        );
        assert!(
            !labels.iter().any(|l| l.contains("typescript.yml")),
            "Bare profile must omit typescript.yml; got: {labels:?}"
        );
        assert!(
            !labels.iter().any(|l| l.contains("rust.yml")),
            "Bare profile must omit rust.yml; got: {labels:?}"
        );
    }

    #[test]
    fn bare_includes_community_files() {
        let profile = RepoProfile {
            languages: Languages::Bare,
            ..RepoProfile::default()
        };
        let labels = plan_labels(&profile);
        assert!(labels.iter().any(|l| l.contains("README.md")));
        assert!(labels.iter().any(|l| l.contains("CONTRIBUTING.md")));
        assert!(labels.iter().any(|l| l.contains("CODE_OF_CONDUCT.md")));
        assert!(labels.iter().any(|l| l.contains("SECURITY.md")));
        assert!(labels.iter().any(|l| l.contains("CHANGELOG.md")));
        assert!(labels.iter().any(|l| l.contains(".coderabbit.yaml")));
    }

    #[test]
    fn bare_has_github_actions_dependabot_only() {
        let profile = RepoProfile {
            languages: Languages::Bare,
            ..RepoProfile::default()
        };
        let labels = plan_labels(&profile);
        let dependabot = labels
            .iter()
            .find(|l| l.contains("dependabot.yml"))
            .cloned()
            .unwrap_or_default();
        assert!(
            dependabot.contains("github-actions"),
            "Bare profile dependabot entry should mention github-actions; got: {dependabot:?}"
        );
        assert!(
            !dependabot.contains("npm"),
            "Bare profile dependabot entry must not mention npm; got: {dependabot:?}"
        );
        assert!(
            !dependabot.contains("cargo"),
            "Bare profile dependabot entry must not mention cargo; got: {dependabot:?}"
        );
    }

    #[test]
    fn license_selection_appears_in_plan() {
        let profile = RepoProfile {
            license: License::Apache2,
            ..RepoProfile::default()
        };
        let labels = plan_labels(&profile);
        assert!(
            labels.iter().any(|l| l.contains("Apache-2.0")),
            "Apache2 license must appear as Apache-2.0 in plan; got: {labels:?}"
        );
    }

    #[test]
    fn validate_rejects_bare_with_publish() {
        let profile = RepoProfile {
            languages: Languages::Bare,
            publish: Publish::Changesets,
            ..RepoProfile::default()
        };
        assert!(
            validate(&profile).is_err(),
            "bare + publish should be rejected"
        );
    }

    #[test]
    fn validate_allows_bare_without_publish() {
        let profile = RepoProfile {
            languages: Languages::Bare,
            publish: Publish::None,
            ..RepoProfile::default()
        };
        assert!(validate(&profile).is_ok());
    }
}
