//! Reusable widget types for the `neon doctor` TUI panes.
//!
//! Each pane implements [`ratatui::widgets::Widget`] so it can be used both
//! by the live `tui_loop` and by the tui-pantry catalog.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    text::{Line, Span, Text},
    widgets::{Block, Paragraph, Widget},
};

use super::{GitIdentity, RepoHealth, ToolInfo};

// --- Tooling pane ---

/// Renders the "Tooling" pane: a table of `{name, version}` tool rows.
pub struct ToolingPane<'a> {
    tools: &'a [ToolInfo],
}

impl<'a> ToolingPane<'a> {
    pub fn new(tools: &'a [ToolInfo]) -> Self {
        Self { tools }
    }
}

impl Widget for ToolingPane<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let lines: Vec<Line> = self
            .tools
            .iter()
            .map(|t| {
                Line::from(vec![
                    Span::raw(format!("  {:>6}  ", t.name)),
                    Span::raw(t.version.clone()),
                ])
            })
            .collect();
        let text = Text::from(lines);
        Paragraph::new(text)
            .block(Block::bordered().title(" Tooling "))
            .render(area, buf);
    }
}

// --- Git Identity pane ---

/// Renders the "Git Identity" pane: user name and email.
pub struct GitIdentityPane<'a> {
    identity: &'a GitIdentity,
}

impl<'a> GitIdentityPane<'a> {
    pub fn new(identity: &'a GitIdentity) -> Self {
        Self { identity }
    }
}

impl Widget for GitIdentityPane<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let text = Text::from(vec![
            Line::from(format!("  name:   {}", self.identity.user_name)),
            Line::from(format!("  email:  {}", self.identity.user_email)),
        ]);
        Paragraph::new(text)
            .block(Block::bordered().title(" Git Identity "))
            .render(area, buf);
    }
}

// --- Repo Health pane ---

/// Renders the "Repo Health" pane: branch, HEAD, and dirty-file count.
pub struct RepoHealthPane<'a> {
    health: &'a RepoHealth,
}

impl<'a> RepoHealthPane<'a> {
    pub fn new(health: &'a RepoHealth) -> Self {
        Self { health }
    }
}

impl Widget for RepoHealthPane<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let text = Text::from(vec![
            Line::from(format!("  branch:  {}", self.health.branch)),
            Line::from(format!("  HEAD:    {}", self.health.short_head)),
            Line::from(format!("  dirty:   {} file(s)", self.health.dirty_count)),
        ]);
        Paragraph::new(text)
            .block(Block::bordered().title(" Repo Health "))
            .render(area, buf);
    }
}

// --- tui-pantry ingredient definitions ---

#[cfg(feature = "pantry")]
pub mod pantry {
    use ratatui::{buffer::Buffer, layout::Rect, widgets::Widget};
    use tui_pantry::Ingredient;

    use crate::doctor::{GitIdentity, RepoHealth, ToolInfo};

    use super::{GitIdentityPane, RepoHealthPane, ToolingPane};

    fn sample_tools() -> Vec<ToolInfo> {
        vec![
            ToolInfo {
                name: "git".to_string(),
                version: "git version 2.49.0".to_string(),
            },
            ToolInfo {
                name: "rustc".to_string(),
                version: "rustc 1.86.0 (05f9846f8 2025-03-31)".to_string(),
            },
            ToolInfo {
                name: "cargo".to_string(),
                version: "cargo 1.86.0 (adf9b6ad1 2025-02-28)".to_string(),
            },
            ToolInfo {
                name: "node".to_string(),
                version: "v22.16.0".to_string(),
            },
            ToolInfo {
                name: "pnpm".to_string(),
                version: "10.11.0".to_string(),
            },
            ToolInfo {
                name: "docker".to_string(),
                version: "Docker version 28.1.1, build 4eba377".to_string(),
            },
        ]
    }

    fn sample_identity() -> GitIdentity {
        GitIdentity {
            user_name: "Ada Lovelace".to_string(),
            user_email: "ada@example.com".to_string(),
        }
    }

    fn sample_health() -> RepoHealth {
        RepoHealth {
            branch: "main".to_string(),
            short_head: "abc1234".to_string(),
            dirty_count: 3,
        }
    }

    // Tooling ingredients

    pub struct ToolingDefault;

    impl Ingredient for ToolingDefault {
        fn group(&self) -> &str {
            "Tooling"
        }
        fn name(&self) -> &str {
            "Default"
        }
        fn source(&self) -> &str {
            "neon_cli::doctor::widgets::ToolingPane"
        }
        fn description(&self) -> &str {
            "All tracked tools present with representative version strings."
        }
        fn render(&self, area: Rect, buf: &mut Buffer) {
            let tools = sample_tools();
            ToolingPane::new(&tools).render(area, buf);
        }
    }

    pub struct ToolingMissing;

    impl Ingredient for ToolingMissing {
        fn group(&self) -> &str {
            "Tooling"
        }
        fn name(&self) -> &str {
            "Missing tools"
        }
        fn source(&self) -> &str {
            "neon_cli::doctor::widgets::ToolingPane"
        }
        fn description(&self) -> &str {
            "Some tools report not-found, showing how missing entries render."
        }
        fn render(&self, area: Rect, buf: &mut Buffer) {
            let tools = vec![
                ToolInfo {
                    name: "git".to_string(),
                    version: "git version 2.49.0".to_string(),
                },
                ToolInfo {
                    name: "rustc".to_string(),
                    version: "not found".to_string(),
                },
                ToolInfo {
                    name: "docker".to_string(),
                    version: "not found".to_string(),
                },
            ];
            ToolingPane::new(&tools).render(area, buf);
        }
    }

    // Git Identity ingredients

    pub struct GitIdentityDefault;

    impl Ingredient for GitIdentityDefault {
        fn group(&self) -> &str {
            "Git Identity"
        }
        fn name(&self) -> &str {
            "Default"
        }
        fn source(&self) -> &str {
            "neon_cli::doctor::widgets::GitIdentityPane"
        }
        fn description(&self) -> &str {
            "User name and email both configured."
        }
        fn render(&self, area: Rect, buf: &mut Buffer) {
            let identity = sample_identity();
            GitIdentityPane::new(&identity).render(area, buf);
        }
    }

    pub struct GitIdentityUnset;

    impl Ingredient for GitIdentityUnset {
        fn group(&self) -> &str {
            "Git Identity"
        }
        fn name(&self) -> &str {
            "Unset"
        }
        fn source(&self) -> &str {
            "neon_cli::doctor::widgets::GitIdentityPane"
        }
        fn description(&self) -> &str {
            "Both name and email unset, showing the (not set) fallback."
        }
        fn render(&self, area: Rect, buf: &mut Buffer) {
            let identity = GitIdentity {
                user_name: "(not set)".to_string(),
                user_email: "(not set)".to_string(),
            };
            GitIdentityPane::new(&identity).render(area, buf);
        }
    }

    // Repo Health ingredients

    pub struct RepoHealthDefault;

    impl Ingredient for RepoHealthDefault {
        fn group(&self) -> &str {
            "Repo Health"
        }
        fn name(&self) -> &str {
            "Default"
        }
        fn source(&self) -> &str {
            "neon_cli::doctor::widgets::RepoHealthPane"
        }
        fn description(&self) -> &str {
            "Branch, HEAD, and a small dirty-file count."
        }
        fn render(&self, area: Rect, buf: &mut Buffer) {
            let health = sample_health();
            RepoHealthPane::new(&health).render(area, buf);
        }
    }

    pub struct RepoHealthClean;

    impl Ingredient for RepoHealthClean {
        fn group(&self) -> &str {
            "Repo Health"
        }
        fn name(&self) -> &str {
            "Clean"
        }
        fn source(&self) -> &str {
            "neon_cli::doctor::widgets::RepoHealthPane"
        }
        fn description(&self) -> &str {
            "Repo with zero dirty files, the clean-slate state."
        }
        fn render(&self, area: Rect, buf: &mut Buffer) {
            let health = RepoHealth {
                branch: "feature/neo-25-pantry".to_string(),
                short_head: "deadbeef".to_string(),
                dirty_count: 0,
            };
            RepoHealthPane::new(&health).render(area, buf);
        }
    }

    /// Returns all doctor-widget ingredients for the tui-pantry catalog.
    pub fn ingredients() -> Vec<Box<dyn tui_pantry::Ingredient>> {
        vec![
            Box::new(ToolingDefault),
            Box::new(ToolingMissing),
            Box::new(GitIdentityDefault),
            Box::new(GitIdentityUnset),
            Box::new(RepoHealthDefault),
            Box::new(RepoHealthClean),
        ]
    }
}
