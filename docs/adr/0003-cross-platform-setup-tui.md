# ADR 0003: Cross-Platform Shell/Terminal Setup as Micro-CLIs + a Thin Orchestrating TUI

## Status

Accepted (design only — build deferred to a later phase)

## Context

Two existing repos provision a developer shell environment:

- `powershell-profile` — Windows / PowerShell 7. A modular profile (9 `lib/Profile.*.psm1`
  modules) plus an idempotent `bootstrap.ps1` installer (winget packages, PSGallery modules,
  corepack pnpm, `$PROFILE` wiring). No Windows-Terminal theme handling yet.
- `zsh-profile` — Linux / WSL. Modular `*.zsh` files plus a `zsh-install` function
  (apt base, eza/delta/lazygit/zoxide, oh-my-posh, oh-my-zsh + plugins, nvm/node/pnpm,
  bat config, obsidian). Several functions are explicitly ports of the PowerShell ones.

Both are already **modular and micro-command-oriented**, and a clear subset is fully
cross-platform (git identity management, docker login, npm/pnpm token, diagnostics) while
the rest is platform-specific install logic. NeonOS wants this provisioning to be a
first-class, repeatable, customizable surface rather than two divergent shell scripts.

## Decision

Model setup as **per-step micro-CLI commands** owned by NeonOS, plus a **thin orchestrating
TUI** that only *selects, configures, and runs the pipeline* — it holds no install logic
itself.

- Each pipeline step is an independently runnable `neon setup <step>` subcommand
  (composable, scriptable, testable in isolation), mirroring how both repos already split
  work into discrete functions.
- The TUI presents the steps, lets the user pick/customize each (which packages, which
  theme, which languages), persists the choices to a config file, then invokes the same
  micro-CLIs in order. TUI = configuration + driver; micro-CLIs = the work.
- Platform is an explicit input. Cross-platform steps share one implementation; platform-
  specific steps dispatch by OS (winget/PSGallery vs apt/brew + release downloads).
- Terminal theming goes through a **windows-terminal-theme-adapter**: a single YAML theme
  is the source of truth, transformed into whatever structure each terminal needs
  (Windows Terminal `settings.json` first; room for others later).

See `docs/architecture/setup-tui.md` for the pipeline, the command map, and the theme schema.

## Consequences

- The two profile repos become **upstream design inputs**, not the shipped mechanism;
  their already-discrete functions port directly onto the micro-CLI surface.
- Idempotency is mandatory per step (detect-then-act, safe re-run) — already the norm in
  both source repos.
- Secrets steps (NPM_TOKEN, SSH keys, docker login) are multi-step and must never hardcode
  or commit secrets; they write to the standard per-tool config locations.
- This is a sizable surface; it is **deferred to a later phase** and tracked as its own
  Linear epic. Phase 1 ships only this design.
