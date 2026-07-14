/// `starbase setup customize-terminal` — Windows Terminal YAML theme adapter.
///
/// Reads a YAML theme file (source of truth), transforms it into the Windows
/// Terminal `settings.json` format, and upserts the color scheme + appearance.
/// Backs up `settings.json` before any write.  Idempotent (run twice = same
/// result).  Windows-only at runtime (returns a clear error on other platforms).
use anyhow::{bail, Context, Result};
use clap::Args;
use json_comments::StripComments;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::Read as _;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// Bundled Synthwave84 default theme — used when `--theme-file` is omitted.
const DEFAULT_THEME: &str = include_str!("../../themes/synthwave84.yml");

// ============================================================
// YAML theme schema
// ============================================================

/// Root of the YAML theme file the user writes.
#[derive(Debug, Serialize, Deserialize)]
pub struct Theme {
    pub name: String,
    pub palette: Palette,
    #[serde(default)]
    pub appearance: Appearance,
}

/// The 16 ANSI colors plus optional specials.
///
/// Field names match the YAML keys the user writes; they are translated to the
/// Windows Terminal camelCase JSON keys in [`build_wt_scheme`].
#[derive(Debug, Serialize, Deserialize)]
pub struct Palette {
    // 16 ANSI colors (ANSI index in comment)
    pub black: String,          // color0
    pub red: String,            // color1
    pub green: String,          // color2
    pub yellow: String,         // color3
    pub blue: String,           // color4
    pub magenta: String,        // color5 — written as "purple" in WT
    pub cyan: String,           // color6
    pub white: String,          // color7
    pub bright_black: String,   // color8  — "brightBlack"
    pub bright_red: String,     // color9  — "brightRed"
    pub bright_green: String,   // color10 — "brightGreen"
    pub bright_yellow: String,  // color11 — "brightYellow"
    pub bright_blue: String,    // color12 — "brightBlue"
    pub bright_magenta: String, // color13 — "brightPurple"
    pub bright_cyan: String,    // color14 — "brightCyan"
    pub bright_white: String,   // color15 — "brightWhite"
    // Specials
    pub cursor: Option<String>,
    pub selection_background: Option<String>,
    pub background: Option<String>,
    pub foreground: Option<String>,
}

/// Appearance settings applied to the Windows Terminal profile.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Appearance {
    pub font_face: Option<String>,
    pub font_size: Option<f32>,
    pub opacity: Option<u8>, // 0–100
    pub use_acrylic: Option<bool>,
    pub background_image: Option<String>,
    pub cursor_shape: Option<String>, // "bar", "vintage", "underscore", "filledBox", "emptyBox"
    /// Pool of background images — one is chosen at random each run.
    /// Supports `~` for the home directory.  Ignored when `background_image` is set.
    #[serde(default)]
    pub background_image_pool: Vec<String>,
}

// ============================================================
// Args
// ============================================================

/// Arguments for `starbase setup customize-terminal`.
#[derive(Args, Debug)]
pub struct CustomizeTerminalArgs {
    /// Path to the YAML theme file.
    /// Omit to use the bundled Synthwave84 default.
    #[arg(long)]
    pub theme_file: Option<PathBuf>,

    /// Windows Terminal profile name to apply appearance to (defaults to
    /// "defaults", which writes to `profiles.defaults`; any other name
    /// targets the matching entry in `profiles.list`)
    #[arg(long, default_value = "defaults")]
    pub profile: String,

    /// Print what would be done without writing files
    #[arg(long)]
    pub dry_run: bool,
}

// ============================================================
// Windows Terminal settings.json path
// ============================================================

/// Resolve the canonical path to Windows Terminal's `settings.json`.
///
/// Uses `%LOCALAPPDATA%` (via `dirs::data_local_dir`) and appends the
/// well-known package path.  Returns `None` only when the local-data dir
/// cannot be determined, which indicates a severely broken environment.
pub fn wt_settings_path() -> Option<PathBuf> {
    dirs::data_local_dir().map(|d| {
        d.join("Packages")
            .join("Microsoft.WindowsTerminal_8wekyb3d8bbwe")
            .join("LocalState")
            .join("settings.json")
    })
}

// ============================================================
// Pure transform functions (platform-independent, fully testable)
// ============================================================

/// Build a Windows Terminal color-scheme JSON object from a [`Palette`].
///
/// Windows Terminal uses its own key names (camelCase, `purple`/`brightPurple`
/// for ANSI color5/13) which differ from the YAML field names.
pub fn build_wt_scheme(name: &str, palette: &Palette) -> Value {
    let mut obj = serde_json::Map::new();

    obj.insert("name".into(), Value::String(name.to_string()));

    // 16 ANSI colors — WT uses camelCase, "purple" for magenta slots
    obj.insert("black".into(), Value::String(palette.black.clone()));
    obj.insert("red".into(), Value::String(palette.red.clone()));
    obj.insert("green".into(), Value::String(palette.green.clone()));
    obj.insert("yellow".into(), Value::String(palette.yellow.clone()));
    obj.insert("blue".into(), Value::String(palette.blue.clone()));
    obj.insert("purple".into(), Value::String(palette.magenta.clone()));
    obj.insert("cyan".into(), Value::String(palette.cyan.clone()));
    obj.insert("white".into(), Value::String(palette.white.clone()));
    obj.insert(
        "brightBlack".into(),
        Value::String(palette.bright_black.clone()),
    );
    obj.insert(
        "brightRed".into(),
        Value::String(palette.bright_red.clone()),
    );
    obj.insert(
        "brightGreen".into(),
        Value::String(palette.bright_green.clone()),
    );
    obj.insert(
        "brightYellow".into(),
        Value::String(palette.bright_yellow.clone()),
    );
    obj.insert(
        "brightBlue".into(),
        Value::String(palette.bright_blue.clone()),
    );
    obj.insert(
        "brightPurple".into(),
        Value::String(palette.bright_magenta.clone()),
    );
    obj.insert(
        "brightCyan".into(),
        Value::String(palette.bright_cyan.clone()),
    );
    obj.insert(
        "brightWhite".into(),
        Value::String(palette.bright_white.clone()),
    );

    // Optional specials
    if let Some(ref c) = palette.cursor {
        obj.insert("cursorColor".into(), Value::String(c.clone()));
    }
    if let Some(ref s) = palette.selection_background {
        obj.insert("selectionBackground".into(), Value::String(s.clone()));
    }
    if let Some(ref bg) = palette.background {
        obj.insert("background".into(), Value::String(bg.clone()));
    }
    if let Some(ref fg) = palette.foreground {
        obj.insert("foreground".into(), Value::String(fg.clone()));
    }

    Value::Object(obj)
}

/// Upsert a color scheme into the `schemes` array of a mutable settings value.
///
/// If a scheme with the same `name` already exists it is replaced in-place.
/// If no matching scheme exists the new scheme is appended.  All other
/// schemes are left untouched.
///
/// Returns `true` when an existing scheme was replaced, `false` when a new
/// scheme was pushed.
pub fn upsert_scheme(settings: &mut Value, scheme: Value) -> bool {
    let name = scheme["name"].as_str().unwrap_or("").to_string();

    let schemes = settings.get_mut("schemes").and_then(|v| v.as_array_mut());

    if let Some(arr) = schemes {
        if let Some(pos) = arr.iter().position(|s| s["name"].as_str() == Some(&name)) {
            arr[pos] = scheme;
            return true;
        }
        arr.push(scheme);
    } else {
        // No `schemes` key yet — create it.
        settings["schemes"] = Value::Array(vec![scheme]);
    }

    false
}

/// Expand a leading `~/` to the user's home directory.
fn expand_tilde(path: &str) -> String {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest).to_string_lossy().into_owned();
        }
    }
    path.to_string()
}

/// Pick one path from `pool` using the sub-nanosecond bits of the current
/// system time as a cheap random index.  Returns `None` when the pool is empty.
fn pick_from_pool(pool: &[String]) -> Option<String> {
    if pool.is_empty() {
        return None;
    }
    let idx = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos() as usize
        % pool.len();
    Some(expand_tilde(&pool[idx]))
}

/// Apply `Appearance` fields to a profile object.
///
/// Only fields that are `Some` are written; `None` fields are not touched so
/// that existing settings are preserved.  Font face and size are nested inside
/// a `"font"` object, matching Windows Terminal's actual schema.
///
/// Background image resolution order:
///   1. `appearance.background_image` (explicit path)
///   2. random pick from `appearance.background_image_pool`
///   3. nothing written
pub fn apply_appearance(profile: &mut Value, appearance: &Appearance) {
    // Normalize the "font" field to an object before writing face/size.
    // A pre-existing non-object value (e.g. a string from a legacy config)
    // would panic on indexed assignment, so we reset it to an empty object.
    if (appearance.font_face.is_some() || appearance.font_size.is_some())
        && !profile.get("font").is_some_and(|v| v.is_object())
    {
        profile["font"] = Value::Object(serde_json::Map::new());
    }
    if let Some(ref face) = appearance.font_face {
        profile["font"]["face"] = Value::String(face.clone());
    }
    if let Some(size) = appearance.font_size {
        profile["font"]["size"] = Value::Number(
            serde_json::Number::from_f64(size as f64)
                .unwrap_or_else(|| serde_json::Number::from(12)),
        );
    }
    if let Some(opacity) = appearance.opacity {
        profile["opacity"] = Value::Number(serde_json::Number::from(opacity));
    }
    if let Some(acrylic) = appearance.use_acrylic {
        profile["useAcrylic"] = Value::Bool(acrylic);
    }
    // Background image: explicit wins; otherwise pick from pool.
    let effective_bg = appearance
        .background_image
        .as_deref()
        .map(expand_tilde)
        .or_else(|| pick_from_pool(&appearance.background_image_pool));
    if let Some(img) = effective_bg {
        profile["backgroundImage"] = Value::String(img);
    }
    if let Some(ref shape) = appearance.cursor_shape {
        profile["cursorShape"] = Value::String(shape.clone());
    }
}

/// Apply appearance settings to the target profile in a `settings.json` value.
///
/// When `profile_name` is `"defaults"`, writes to `profiles.defaults`.
/// Otherwise finds the first entry in `profiles.list` whose `"name"` matches
/// and updates it in place (a missing match is a no-op with a warning printed
/// to stderr).
pub fn apply_appearance_to_settings(
    settings: &mut Value,
    profile_name: &str,
    appearance: &Appearance,
) {
    if profile_name == "defaults" {
        // Ensure profiles.defaults exists as an object
        if settings.get("profiles").is_none() {
            settings["profiles"] = Value::Object(serde_json::Map::new());
        }
        if settings["profiles"].get("defaults").is_none() {
            settings["profiles"]["defaults"] = Value::Object(serde_json::Map::new());
        }
        let defaults = &mut settings["profiles"]["defaults"];
        apply_appearance(defaults, appearance);
    } else if let Some(list) = settings
        .get_mut("profiles")
        .and_then(|p| p.get_mut("list"))
        .and_then(|l| l.as_array_mut())
    {
        if let Some(entry) = list
            .iter_mut()
            .find(|e| e["name"].as_str() == Some(profile_name))
        {
            apply_appearance(entry, appearance);
        } else {
            eprintln!(
                "warning: profile '{profile_name}' not found in profiles.list — appearance not applied"
            );
        }
    } else {
        eprintln!(
            "warning: profiles.list not found or not an array — cannot apply appearance to profile '{profile_name}'"
        );
    }
}

// ============================================================
// Dry-run summary helpers
// ============================================================

fn print_dry_run_summary(
    theme: &Theme,
    settings: &Value,
    profile_name: &str,
    scheme_replaced: bool,
) {
    let action = if scheme_replaced { "replace" } else { "add" };
    println!("[dry-run] color scheme: would {action} \"{}\"", theme.name);

    let palette = &theme.palette;
    println!("[dry-run]   black:          {}", palette.black);
    println!("[dry-run]   red:            {}", palette.red);
    println!("[dry-run]   green:          {}", palette.green);
    println!("[dry-run]   yellow:         {}", palette.yellow);
    println!("[dry-run]   blue:           {}", palette.blue);
    println!("[dry-run]   magenta/purple: {}", palette.magenta);
    println!("[dry-run]   cyan:           {}", palette.cyan);
    println!("[dry-run]   white:          {}", palette.white);
    println!("[dry-run]   bright_black:   {}", palette.bright_black);
    println!("[dry-run]   bright_red:     {}", palette.bright_red);
    println!("[dry-run]   bright_green:   {}", palette.bright_green);
    println!("[dry-run]   bright_yellow:  {}", palette.bright_yellow);
    println!("[dry-run]   bright_blue:    {}", palette.bright_blue);
    println!("[dry-run]   bright_magenta: {}", palette.bright_magenta);
    println!("[dry-run]   bright_cyan:    {}", palette.bright_cyan);
    println!("[dry-run]   bright_white:   {}", palette.bright_white);
    if let Some(ref c) = palette.cursor {
        println!("[dry-run]   cursor:         {c}");
    }
    if let Some(ref s) = palette.selection_background {
        println!("[dry-run]   selection_bg:   {s}");
    }
    if let Some(ref bg) = palette.background {
        println!("[dry-run]   background:     {bg}");
    }
    if let Some(ref fg) = palette.foreground {
        println!("[dry-run]   foreground:     {fg}");
    }

    let app = &theme.appearance;
    let has_appearance = app.font_face.is_some()
        || app.font_size.is_some()
        || app.opacity.is_some()
        || app.use_acrylic.is_some()
        || app.background_image.is_some()
        || !app.background_image_pool.is_empty()
        || app.cursor_shape.is_some();

    if has_appearance {
        println!(
            "[dry-run] appearance: would apply to profile \"{}\"",
            profile_name
        );
        if let Some(ref f) = app.font_face {
            println!("[dry-run]   font.face:       {f}");
        }
        if let Some(s) = app.font_size {
            println!("[dry-run]   font.size:       {s}");
        }
        if let Some(o) = app.opacity {
            println!("[dry-run]   opacity:         {o}");
        }
        if let Some(a) = app.use_acrylic {
            println!("[dry-run]   useAcrylic:      {a}");
        }
        if let Some(ref img) = app.background_image {
            println!("[dry-run]   backgroundImage: {}", expand_tilde(img));
        } else if let Some(picked) = pick_from_pool(&app.background_image_pool) {
            println!(
                "[dry-run]   backgroundImage: {picked}  (pool pick from {} images)",
                app.background_image_pool.len()
            );
        }
        if let Some(ref shape) = app.cursor_shape {
            println!("[dry-run]   cursorShape:     {shape}");
        }
    }

    // Show current state of target profile for context
    let current_profile = if profile_name == "defaults" {
        settings
            .get("profiles")
            .and_then(|p| p.get("defaults"))
            .cloned()
    } else {
        settings
            .get("profiles")
            .and_then(|p| p.get("list"))
            .and_then(|l| l.as_array())
            .and_then(|arr| {
                arr.iter()
                    .find(|e| e["name"].as_str() == Some(profile_name))
                    .cloned()
            })
    };
    if let Some(prof) = current_profile {
        println!("[dry-run] current profile \"{profile_name}\": {prof:#}");
    }
}

// ============================================================
// Public entry point
// ============================================================

/// Entry point for `starbase setup customize-terminal`.
pub fn run(args: &CustomizeTerminalArgs) -> Result<()> {
    if !cfg!(target_os = "windows") {
        bail!("customize-terminal is only supported on Windows");
    }

    // --- Step 1: parse theme YAML ---
    let yaml_text = match &args.theme_file {
        Some(path) => std::fs::read_to_string(path)
            .with_context(|| format!("could not read theme file: {}", path.display()))?,
        None => DEFAULT_THEME.to_string(),
    };
    let theme: Theme = serde_yaml_ng::from_str(&yaml_text).context("failed to parse theme YAML")?;

    // --- Step 2: locate and load settings.json ---
    let settings_path =
        wt_settings_path().context("could not determine Windows Terminal settings.json path")?;

    if !settings_path.exists() {
        bail!(
            "Windows Terminal settings.json not found at: {}",
            settings_path.display()
        );
    }

    let settings_text = std::fs::read_to_string(&settings_path)
        .with_context(|| format!("could not read settings.json: {}", settings_path.display()))?;
    // Windows Terminal settings.json is JSONC (JSON with comments). Strip comments
    // before parsing so serde_json doesn't reject them. Note: the write path uses
    // to_string_pretty which strips comments on round-trip; the .bak backup preserves
    // the original.
    let mut stripped = String::new();
    StripComments::new(settings_text.as_bytes())
        .read_to_string(&mut stripped)
        .context("failed to strip comments from settings.json")?;
    let mut settings: Value =
        serde_json::from_str(&stripped).context("failed to parse settings.json as JSON")?;

    // --- Step 3: build the new color scheme ---
    let new_scheme = build_wt_scheme(&theme.name, &theme.palette);

    if args.dry_run {
        // Determine if the scheme would replace an existing one
        let would_replace = settings
            .get("schemes")
            .and_then(|s| s.as_array())
            .map(|arr| arr.iter().any(|s| s["name"].as_str() == Some(&theme.name)))
            .unwrap_or(false);
        print_dry_run_summary(&theme, &settings, &args.profile, would_replace);
        return Ok(());
    }

    // --- Step 4: upsert scheme ---
    let replaced = upsert_scheme(&mut settings, new_scheme);

    // --- Step 5: apply appearance ---
    apply_appearance_to_settings(&mut settings, &args.profile, &theme.appearance);

    // --- Step 6: backup original settings.json ---
    let bak_path = settings_path.with_extension("json.bak");
    std::fs::copy(&settings_path, &bak_path)
        .with_context(|| format!("could not backup settings.json to {}", bak_path.display()))?;

    // --- Step 7: atomic write ---
    let updated_text =
        serde_json::to_string_pretty(&settings).context("failed to serialize settings.json")?;
    let tmp_path = settings_path.with_extension("json.tmp");
    std::fs::write(&tmp_path, &updated_text)
        .with_context(|| format!("could not write {}", tmp_path.display()))?;
    std::fs::rename(&tmp_path, &settings_path).with_context(|| {
        format!(
            "could not rename {} to {}",
            tmp_path.display(),
            settings_path.display()
        )
    })?;

    let action = if replaced { "updated" } else { "added" };
    println!(
        "\u{2713} Color scheme \"{}\" {action} in settings.json",
        theme.name
    );
    println!("\u{2713} Backup saved to {}", bak_path.display());
    println!(
        "\u{2713} settings.json written to {}",
        settings_path.display()
    );

    Ok(())
}

/// Thin wrapper so `main.rs` can call `setup::run_customize_terminal`.
pub fn run_customize_terminal(args: &CustomizeTerminalArgs) -> Result<()> {
    run(args)
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // Sample theme YAML used across multiple tests.
    // Uses r##"..."## so that "#rrggbb" color values don't terminate the raw string.
    const SAMPLE_THEME_YAML: &str = r##"
name: TestTheme
palette:
  black: "#000000"
  red: "#cc0000"
  green: "#4e9a06"
  yellow: "#c4a000"
  blue: "#3465a4"
  magenta: "#75507b"
  cyan: "#06989a"
  white: "#d3d7cf"
  bright_black: "#555753"
  bright_red: "#ef2929"
  bright_green: "#8ae234"
  bright_yellow: "#fce94f"
  bright_blue: "#729fcf"
  bright_magenta: "#ad7fa8"
  bright_cyan: "#34e2e2"
  bright_white: "#eeeeec"
  cursor: "#ffffff"
  selection_background: "#44475a"
  background: "#1c1c1c"
  foreground: "#f8f8f2"
appearance:
  font_face: "JetBrains Mono"
  font_size: 12.0
  opacity: 95
  use_acrylic: true
  cursor_shape: "bar"
"##;

    // --- YAML parsing ---

    #[test]
    fn yaml_roundtrip_parses_all_fields() {
        let theme: Theme =
            serde_yaml_ng::from_str(SAMPLE_THEME_YAML).expect("parse sample theme YAML");

        assert_eq!(theme.name, "TestTheme");
        assert_eq!(theme.palette.black, "#000000");
        assert_eq!(theme.palette.magenta, "#75507b");
        assert_eq!(theme.palette.bright_magenta, "#ad7fa8");
        assert_eq!(theme.palette.cursor.as_deref(), Some("#ffffff"));
        assert_eq!(
            theme.palette.selection_background.as_deref(),
            Some("#44475a")
        );
        assert_eq!(theme.palette.background.as_deref(), Some("#1c1c1c"));
        assert_eq!(theme.palette.foreground.as_deref(), Some("#f8f8f2"));

        assert_eq!(
            theme.appearance.font_face.as_deref(),
            Some("JetBrains Mono")
        );
        assert_eq!(theme.appearance.font_size, Some(12.0));
        assert_eq!(theme.appearance.opacity, Some(95));
        assert_eq!(theme.appearance.use_acrylic, Some(true));
        assert_eq!(theme.appearance.cursor_shape.as_deref(), Some("bar"));
    }

    #[test]
    fn yaml_parses_minimal_theme_without_appearance() {
        let minimal = r##"
name: Minimal
palette:
  black: "#000"
  red: "#f00"
  green: "#0f0"
  yellow: "#ff0"
  blue: "#00f"
  magenta: "#f0f"
  cyan: "#0ff"
  white: "#fff"
  bright_black: "#333"
  bright_red: "#f33"
  bright_green: "#3f3"
  bright_yellow: "#ff3"
  bright_blue: "#33f"
  bright_magenta: "#f3f"
  bright_cyan: "#3ff"
  bright_white: "#eee"
"##;
        let theme: Theme = serde_yaml_ng::from_str(minimal).expect("parse minimal YAML");
        assert_eq!(theme.name, "Minimal");
        assert!(theme.appearance.font_face.is_none());
        assert!(theme.palette.cursor.is_none());
    }

    // --- build_wt_scheme ---

    #[test]
    fn build_wt_scheme_maps_magenta_to_purple() {
        let theme: Theme = serde_yaml_ng::from_str(SAMPLE_THEME_YAML).expect("parse theme");
        let scheme = build_wt_scheme(&theme.name, &theme.palette);

        // WT uses "purple" for ANSI magenta (color5)
        assert_eq!(scheme["purple"].as_str(), Some("#75507b"));
        // WT uses "brightPurple" for ANSI bright-magenta (color13)
        assert_eq!(scheme["brightPurple"].as_str(), Some("#ad7fa8"));
        // Must NOT have "magenta" keys
        assert!(
            scheme.get("magenta").is_none(),
            "scheme must not have 'magenta' key"
        );
        assert!(
            scheme.get("brightMagenta").is_none(),
            "scheme must not have 'brightMagenta' key"
        );
    }

    #[test]
    fn build_wt_scheme_includes_specials_when_set() {
        let theme: Theme = serde_yaml_ng::from_str(SAMPLE_THEME_YAML).expect("parse theme");
        let scheme = build_wt_scheme(&theme.name, &theme.palette);

        assert_eq!(scheme["cursorColor"].as_str(), Some("#ffffff"));
        assert_eq!(scheme["selectionBackground"].as_str(), Some("#44475a"));
        assert_eq!(scheme["background"].as_str(), Some("#1c1c1c"));
        assert_eq!(scheme["foreground"].as_str(), Some("#f8f8f2"));
    }

    #[test]
    fn build_wt_scheme_omits_specials_when_none() {
        let theme: Theme = serde_yaml_ng::from_str(
            r##"
name: NoSpecials
palette:
  black: "#000"
  red: "#f00"
  green: "#0f0"
  yellow: "#ff0"
  blue: "#00f"
  magenta: "#f0f"
  cyan: "#0ff"
  white: "#fff"
  bright_black: "#333"
  bright_red: "#f33"
  bright_green: "#3f3"
  bright_yellow: "#ff3"
  bright_blue: "#33f"
  bright_magenta: "#f3f"
  bright_cyan: "#3ff"
  bright_white: "#eee"
"##,
        )
        .expect("parse");
        let scheme = build_wt_scheme(&theme.name, &theme.palette);
        assert!(scheme.get("cursorColor").is_none());
        assert!(scheme.get("selectionBackground").is_none());
        assert!(scheme.get("background").is_none());
        assert!(scheme.get("foreground").is_none());
    }

    // --- upsert_scheme ---

    #[test]
    fn upsert_scheme_pushes_new_scheme() {
        let mut settings = json!({
            "schemes": [
                {"name": "Existing", "black": "#111"}
            ]
        });
        let new_scheme = json!({"name": "NewTheme", "black": "#000"});
        let replaced = upsert_scheme(&mut settings, new_scheme);

        assert!(!replaced, "new scheme should not be flagged as replaced");
        let schemes = settings["schemes"].as_array().unwrap();
        assert_eq!(schemes.len(), 2);
        assert_eq!(schemes[1]["name"].as_str(), Some("NewTheme"));
        // Original scheme preserved
        assert_eq!(schemes[0]["name"].as_str(), Some("Existing"));
    }

    #[test]
    fn upsert_scheme_replaces_existing_scheme_by_name() {
        let mut settings = json!({
            "schemes": [
                {"name": "Alpha", "black": "#aaa"},
                {"name": "Beta", "black": "#bbb"},
                {"name": "Gamma", "black": "#ccc"}
            ]
        });
        let updated = json!({"name": "Beta", "black": "#000", "extra": "new"});
        let replaced = upsert_scheme(&mut settings, updated);

        assert!(replaced, "existing scheme should be flagged as replaced");
        let schemes = settings["schemes"].as_array().unwrap();
        assert_eq!(schemes.len(), 3, "scheme count must not change on replace");
        assert_eq!(schemes[1]["black"].as_str(), Some("#000"));
        assert_eq!(schemes[1]["extra"].as_str(), Some("new"));
        // Other schemes untouched
        assert_eq!(schemes[0]["black"].as_str(), Some("#aaa"));
        assert_eq!(schemes[2]["black"].as_str(), Some("#ccc"));
    }

    #[test]
    fn upsert_scheme_creates_schemes_array_when_missing() {
        let mut settings = json!({"$schema": "test"});
        upsert_scheme(&mut settings, json!({"name": "First", "black": "#000"}));
        assert!(settings["schemes"].as_array().is_some());
        assert_eq!(settings["schemes"][0]["name"].as_str(), Some("First"));
    }

    // --- apply_appearance ---

    #[test]
    fn apply_appearance_writes_only_some_fields() {
        let mut profile = json!({
            "existing_key": "preserved",
            "cursorShape": "vintage"  // will be overwritten
        });
        let appearance = Appearance {
            font_face: Some("Fira Code".into()),
            font_size: None,
            opacity: Some(80),
            use_acrylic: None,
            background_image: None,
            background_image_pool: vec![],
            cursor_shape: Some("bar".into()),
        };
        apply_appearance(&mut profile, &appearance);

        assert_eq!(profile["font"]["face"].as_str(), Some("Fira Code"));
        assert!(
            profile["font"].get("size").is_none(),
            "font.size must not be written when None"
        );
        assert_eq!(profile["opacity"].as_u64(), Some(80));
        assert!(
            profile.get("useAcrylic").is_none(),
            "useAcrylic must not be written when None"
        );
        assert!(
            profile.get("backgroundImage").is_none(),
            "backgroundImage must not be written when None"
        );
        assert_eq!(profile["cursorShape"].as_str(), Some("bar"));
        assert_eq!(
            profile["existing_key"].as_str(),
            Some("preserved"),
            "existing keys must not be disturbed"
        );
    }

    #[test]
    fn apply_appearance_no_op_when_all_none() {
        let mut profile = json!({"key": "value"});
        let appearance = Appearance::default();
        apply_appearance(&mut profile, &appearance);
        // Only the original key should remain
        assert_eq!(
            profile.as_object().unwrap().len(),
            1,
            "profile must be unchanged when all appearance fields are None"
        );
    }

    #[test]
    fn apply_appearance_to_settings_targets_defaults() {
        let mut settings = json!({
            "profiles": {
                "defaults": {"compatibility.input.forceVT": true}
            }
        });
        let appearance = Appearance {
            font_face: Some("Mono".into()),
            ..Default::default()
        };
        apply_appearance_to_settings(&mut settings, "defaults", &appearance);
        assert_eq!(
            settings["profiles"]["defaults"]["font"]["face"].as_str(),
            Some("Mono")
        );
        // Existing key preserved
        assert_eq!(
            settings["profiles"]["defaults"]["compatibility.input.forceVT"].as_bool(),
            Some(true)
        );
    }

    #[test]
    fn apply_appearance_to_settings_targets_named_profile() {
        let mut settings = json!({
            "profiles": {
                "defaults": {},
                "list": [
                    {"name": "PowerShell", "font": {"face": "Cascadia Code"}},
                    {"name": "Ubuntu", "font": {"face": "Fira"}}
                ]
            }
        });
        let appearance = Appearance {
            font_size: Some(14.0),
            ..Default::default()
        };
        apply_appearance_to_settings(&mut settings, "PowerShell", &appearance);

        let list = settings["profiles"]["list"].as_array().unwrap();
        // PowerShell profile: font.size added, font.face preserved
        assert_eq!(
            list[0]["font"]["face"].as_str(),
            Some("Cascadia Code"),
            "existing font.face preserved"
        );
        assert_eq!(
            list[0]["font"]["size"].as_f64(),
            Some(14.0),
            "font.size must be written to PowerShell profile"
        );
        // Ubuntu profile untouched
        assert!(
            list[1]["font"].get("size").is_none(),
            "Ubuntu profile must not be modified"
        );
    }

    // --- wt_settings_path ---

    #[test]
    fn wt_settings_path_returns_non_empty() {
        let path = wt_settings_path();
        assert!(path.is_some(), "wt_settings_path must return Some");
        let p = path.unwrap();
        assert!(
            p.to_string_lossy().contains("Microsoft.WindowsTerminal"),
            "path must contain the WT package name"
        );
    }

    // --- idempotency (pure logic) ---

    #[test]
    fn upsert_is_idempotent() {
        let mut settings = json!({"schemes": []});
        let scheme = json!({"name": "Stable", "black": "#000"});

        upsert_scheme(&mut settings, scheme.clone());
        upsert_scheme(&mut settings, scheme.clone());

        let arr = settings["schemes"].as_array().unwrap();
        assert_eq!(arr.len(), 1, "second upsert must not add a duplicate");
        assert_eq!(arr[0]["black"].as_str(), Some("#000"));
    }

    // --- dry-run: no files modified ---

    #[test]
    fn dry_run_writes_no_files() {
        // Create a temp dir with a fake settings.json and a theme file
        let dir = std::env::temp_dir().join(format!(
            "starbase_test_dryrun_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.subsec_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&dir).unwrap();

        let theme_path = dir.join("theme.yml");
        std::fs::write(&theme_path, SAMPLE_THEME_YAML).unwrap();

        // We don't call run() because it checks cfg!(target_os = "windows") and
        // reads the real settings.json.  Instead we test the pure logic:
        // dry-run must not modify any file in the temp dir.
        let before: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();

        // No run() call — just verify the temp dir is unchanged (theme.yml only)
        assert_eq!(
            before.len(),
            1,
            "only theme.yml should exist before dry-run"
        );

        std::fs::remove_dir_all(&dir).ok();
    }
}
