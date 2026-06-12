use anyhow::Result;
use std::io::IsTerminal;
use std::process::Command;

pub mod widgets;

// --- Data collection ---

pub(crate) struct ToolInfo {
    pub(crate) name: String,
    pub(crate) version: String,
}

pub(crate) struct GitIdentity {
    pub(crate) user_name: String,
    pub(crate) user_email: String,
}

pub(crate) struct RepoHealth {
    pub(crate) branch: String,
    pub(crate) short_head: String,
    pub(crate) dirty_count: usize,
}

struct DiagData {
    tools: Vec<ToolInfo>,
    identity: GitIdentity,
    health: RepoHealth,
}

/// Build a `Command` that can resolve `.cmd`/`.bat` shims on Windows.
///
/// On Windows, `Command::new(program)` only finds the bare executable name on
/// PATH — it does not consult PATHEXT, so corepack shims like `pnpm.cmd` are
/// invisible. Routing through `cmd /c` gives us the shell's full PATHEXT
/// resolution at essentially no extra cost.
///
/// On non-Windows platforms we return a direct `Command::new(program)` so the
/// behaviour and performance are identical to the old code.
#[cfg(windows)]
fn make_command(program: &str) -> Command {
    let mut cmd = Command::new("cmd");
    // `/d` disables AutoRun registry commands so they can't inject extra output
    // or side effects into the probe; `/c` runs the program and exits.
    cmd.args(["/d", "/c", program]);
    cmd
}

#[cfg(not(windows))]
fn make_command(program: &str) -> Command {
    Command::new(program)
}

/// Run a command and return trimmed stdout, or an error string on any failure.
fn run(program: &str, args: &[&str]) -> String {
    match make_command(program).args(args).output() {
        Err(_) => "not found".to_string(),
        Ok(out) => {
            if out.status.success() {
                String::from_utf8_lossy(&out.stdout).trim().to_string()
            } else if out.status.code() == Some(9009) {
                // Windows `cmd /c <missing>` exits 9009 ("'x' is not recognized").
                // Preserve the documented "not found" UX for absent tools rather
                // than surfacing the shell's error text.
                "not found".to_string()
            } else {
                // Treat non-zero exit as not available / unknown
                let stderr = String::from_utf8_lossy(&out.stderr);
                let trimmed = stderr.trim();
                if trimmed.is_empty() {
                    "not found".to_string()
                } else {
                    format!("error: {trimmed}")
                }
            }
        }
    }
}

/// Extract the first line from a version string (most tools print extra lines).
fn first_line(s: &str) -> String {
    s.lines().next().unwrap_or(s).to_string()
}

/// Map a `run()` result to "(not set)" when the value is missing, empty, or errored.
/// `git config user.name`/`user.email` exit non-zero when unset, which `run()` turns
/// into "not found"/"error: ...", so treat those as unset rather than displaying them.
fn or_not_set(value: String) -> String {
    if value.is_empty() || value == "not found" || value.starts_with("error") {
        "(not set)".to_string()
    } else {
        value
    }
}

fn collect() -> DiagData {
    // Tool versions -- ask each tool for its version string.
    let tools = vec![
        ToolInfo {
            name: "git".to_string(),
            version: first_line(&run("git", &["--version"])),
        },
        ToolInfo {
            name: "rustc".to_string(),
            version: first_line(&run("rustc", &["--version"])),
        },
        ToolInfo {
            name: "cargo".to_string(),
            version: first_line(&run("cargo", &["--version"])),
        },
        ToolInfo {
            name: "node".to_string(),
            version: first_line(&run("node", &["--version"])),
        },
        ToolInfo {
            name: "pnpm".to_string(),
            version: first_line(&run("pnpm", &["--version"])),
        },
        ToolInfo {
            name: "docker".to_string(),
            version: first_line(&run("docker", &["--version"])),
        },
    ];

    // Git identity (unset config exits non-zero -> treat as "(not set)")
    let identity = GitIdentity {
        user_name: or_not_set(run("git", &["config", "user.name"])),
        user_email: or_not_set(run("git", &["config", "user.email"])),
    };

    // Repo health
    let branch = {
        let b = run("git", &["rev-parse", "--abbrev-ref", "HEAD"]);
        if b == "not found" || b.starts_with("error") {
            "(not a git repo)".to_string()
        } else {
            b
        }
    };

    let short_head = {
        let h = run("git", &["rev-parse", "--short", "HEAD"]);
        if h == "not found" || h.starts_with("error") {
            "(unknown)".to_string()
        } else {
            h
        }
    };

    let dirty_count = {
        let out = run("git", &["status", "--porcelain"]);
        if out == "not found" || out.starts_with("error") || out.is_empty() {
            0
        } else {
            out.lines().count()
        }
    };

    let health = RepoHealth {
        branch,
        short_head,
        dirty_count,
    };

    DiagData {
        tools,
        identity,
        health,
    }
}

// --- Plain-text report (non-interactive / CI) ---

fn format_plain(data: &DiagData) -> String {
    use std::fmt::Write;
    let mut s = String::new();

    let _ = writeln!(s, "=== Tooling ===");
    for t in &data.tools {
        let _ = writeln!(s, "  {:>6}: {}", t.name, t.version);
    }

    let _ = writeln!(s);
    let _ = writeln!(s, "=== Git Identity ===");
    let _ = writeln!(s, "  name:  {}", data.identity.user_name);
    let _ = writeln!(s, "  email: {}", data.identity.user_email);

    let _ = writeln!(s);
    let _ = writeln!(s, "=== Repo Health ===");
    let _ = writeln!(s, "  branch:    {}", data.health.branch);
    let _ = writeln!(s, "  HEAD:      {}", data.health.short_head);
    let _ = writeln!(s, "  dirty:     {} file(s)", data.health.dirty_count);

    s
}

fn print_plain(data: &DiagData) {
    print!("{}", format_plain(data));
}

// --- TUI (interactive) ---

fn run_tui(data: &DiagData) -> Result<()> {
    let mut terminal = ratatui::init();
    let result = tui_loop(&mut terminal, data);
    // Always restore -- ignore restore errors so we can surface the loop error.
    ratatui::restore();
    result?;
    Ok(())
}

fn tui_loop(terminal: &mut ratatui::DefaultTerminal, data: &DiagData) -> Result<()> {
    use crossterm::event::{self, KeyCode, KeyEventKind};
    use ratatui::layout::{Constraint, Direction, Layout};

    use widgets::{GitIdentityPane, RepoHealthPane, ToolingPane};

    loop {
        terminal.draw(|frame| {
            let area = frame.area();

            // Outer layout: content + footer
            let outer = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(0), Constraint::Length(1)])
                .split(area);

            // Three panes stacked vertically
            let panes = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Percentage(40),
                    Constraint::Percentage(25),
                    Constraint::Percentage(35),
                ])
                .split(outer[0]);

            // Pane 1 -- Tooling
            frame.render_widget(ToolingPane::new(&data.tools), panes[0]);

            // Pane 2 -- Git Identity
            frame.render_widget(GitIdentityPane::new(&data.identity), panes[1]);

            // Pane 3 -- Repo Health
            frame.render_widget(RepoHealthPane::new(&data.health), panes[2]);

            // Footer hint
            use ratatui::widgets::Paragraph;
            let footer = Paragraph::new("  q / Esc  quit");
            frame.render_widget(footer, outer[1]);
        })?;

        if event::poll(std::time::Duration::from_millis(100))? {
            if let event::Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                        _ => {}
                    }
                }
            }
        }
    }
}

// --- Public entry point ---

pub fn gather() -> Result<()> {
    let data = collect();

    if std::io::stdout().is_terminal() {
        run_tui(&data)
    } else {
        print_plain(&data);
        Ok(())
    }
}

// --- Tests ---

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_line_takes_only_the_first_line() {
        assert_eq!(
            first_line("git version 2.43.0\nextra\nlines"),
            "git version 2.43.0"
        );
        assert_eq!(first_line("single"), "single");
        assert_eq!(first_line(""), "");
    }

    #[test]
    fn format_plain_renders_all_sections() {
        let data = DiagData {
            tools: vec![ToolInfo {
                name: "git".to_string(),
                version: "git version 2.43.0".to_string(),
            }],
            identity: GitIdentity {
                user_name: "Ada".to_string(),
                user_email: "ada@example.com".to_string(),
            },
            health: RepoHealth {
                branch: "main".to_string(),
                short_head: "abc1234".to_string(),
                dirty_count: 2,
            },
        };

        let report = format_plain(&data);
        assert!(report.contains("=== Tooling ==="));
        assert!(report.contains("git: git version 2.43.0"));
        assert!(report.contains("=== Git Identity ==="));
        assert!(report.contains("ada@example.com"));
        assert!(report.contains("=== Repo Health ==="));
        assert!(report.contains("branch:    main"));
        assert!(report.contains("dirty:     2 file(s)"));
    }

    #[test]
    fn run_rustc_version_resolves_on_this_platform() {
        // `rustc` must exist to compile/run this test binary, so it's a more
        // reliable probe than `git` (absent in minimal containers/tarballs).
        // This exercises the make_command path end-to-end.
        let out = run("rustc", &["--version"]);
        assert!(
            out.starts_with("rustc "),
            "expected 'rustc ...', got: {out}"
        );
    }
}
