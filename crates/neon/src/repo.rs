use anyhow::Result;
use clap::{Args, ValueEnum};
use std::path::PathBuf;

// --- Profile inputs (§7 of docs/architecture/repo-setup-automation.md) ---

/// Repository topology: single-maintainer vs multi-person team.
///
/// Affects GitHub branch-protection settings applied by `neon repo harden`
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
    /// Both TypeScript and Rust (the canonical NeonOS profile).
    #[default]
    Both,
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
}

// --- CLI args struct ---

/// Arguments for `neon repo init`.
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

/// Derive the list of files / artifacts that `neon repo init` *would* generate for
/// the given profile.  Pure function — no filesystem access, no I/O.
pub fn plan(profile: &RepoProfile) -> Vec<PlanItem> {
    // Start with language-independent community & docs files; extend conditionally.
    let mut items: Vec<PlanItem> = vec![
        PlanItem::new("README.md"),
        PlanItem::new("LICENSE (MIT)"),
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
    ];

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

    let _ = writeln!(s, "=== neon repo init — DRY RUN ===");
    let _ = writeln!(s, "  target:     {target}");
    let _ = writeln!(s);
    let _ = writeln!(s, "=== Profile ===");
    let _ = writeln!(s, "  topology:   {:?}", profile.topology);
    let _ = writeln!(s, "  visibility: {:?}", profile.visibility);
    let _ = writeln!(s, "  publish:    {:?}", profile.publish);
    let _ = writeln!(s, "  merge:      {:?}", profile.merge);
    let _ = writeln!(s, "  languages:  {:?}", profile.languages);
    let _ = writeln!(s);
    let _ = writeln!(s, "=== Would generate ===");

    for item in plan(profile) {
        let _ = writeln!(s, "  would create: {}", item.label);
    }

    let _ = writeln!(s);
    let _ = writeln!(s, "No files were written (dry-run).");

    s
}

// --- Public entry point ---

/// Entry point for `neon repo init`.
///
/// Builds a `RepoProfile` from the parsed CLI arguments and prints a
/// human-readable scaffold plan to stdout.  No files are written and no
/// network or git calls are made in this slice.
pub fn init(args: InitArgs) -> Result<()> {
    let path = args.path.clone();
    let profile = args.into_profile();
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
    fn format_plan_contains_profile_summary_and_items() {
        let profile = RepoProfile::default();
        let output = format_plan(&profile, None);
        assert!(output.contains("=== neon repo init"));
        assert!(output.contains("=== Profile ==="));
        assert!(output.contains("=== Would generate ==="));
        assert!(output.contains("would create:"));
        assert!(output.contains("No files were written (dry-run)."));
        assert!(output.contains("target:     ."));
    }
}
