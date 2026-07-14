# Architecture: Cross-Platform Setup TUI (`starbase setup`)

> Design note. Implements ADR 0003. **Build active** — workspace provisioning is the
> priority workstream, built in parallel with repo-init. Inputs:
> `powershell-profile` (Windows/PowerShell 7) and `zsh-profile` (Linux/WSL).

## Shape

```sh
starbase setup            # launch the TUI: pick + customize each step, then run the pipeline
starbase setup <step>     # run one step headless (idempotent, scriptable)
starbase setup run        # run the full configured pipeline non-interactively (CI / re-provision)
```

The **TUI is a thin driver**: it selects platform/options per step, writes a config file,
and invokes the **micro-CLIs** in order. All real work lives in the per-step subcommands.
Config persisted to e.g. `~/.config/starbase/setup.toml` (the declarative desired state; re-runs
converge to it).

## Pipeline (the user-facing order)

1. **Detect machine / OS** — probe the host platform (OS family, architecture, distro) and
   persist the result as the context for every downstream step. (NEO-46)
2. **Pick a shell** — Windows: PowerShell 7 or WSL. (Drives every platform-specific step after.)
3. **Pick a terminal** — Windows Terminal for now (extensible).
4. **Customize the terminal** — settings, background, theme → via the theme adapter (below).
5. **Install languages** — nvm→node, python, rust (each toggleable / version-pinnable).
6. **Install core apps** — required-by-everything: git, docker, obsidian, …
7. **Install packages** — zsh, oh-my-zsh/oh-my-posh, posh-git, lazygit, fzf, bat, zoxide, …
8. **Select custom functions / aliases** — the reusable helpers from the profile repos.
9. **Set environment variables + secrets** — multi-step: NPM_TOKEN, `.ssh` for git,
   docker login, … (never hardcoded; written to standard per-tool locations).
10. **Set up the shell profile** — wire `$PROFILE` / `.zshrc` to the Starbase-managed profile.
11. **Initialize Claude / agent environment** — claude-config machine bootstrap: links
    (symlinks on Unix, junctions on Windows) for `~/.claude/skills` + `~/.claude/agents`,
    run sync-skills, provision global
    `~/.claude/CLAUDE.md` + `local-config.md`. Per-machine; distinct from per-repo agent
    setup (NEO-3). (NEO-47)
12. **Done** — final diagnostics: detect anything missing and report.

## Micro-CLI command map

Reuse the already-discrete functions from both source repos. Two classes:

### Cross-platform (one implementation, ~8)
Direct ports of functions that already exist on both sides:

| Step | Source (PowerShell → zsh) |
|---|---|
| `detect` (platform probe: OS family, arch, distro) | NEO-46 — new |
| `claude` (claude-config bootstrap: links (symlinks/junctions), sync-skills, global config) | NEO-47 — new |
| `set-git-identity` (local, remote-rewrite to SSH alias) | `Set-LocalGitIdentity` → `set-git-identity` |
| `set-global-git-identity` | `Set-GlobalGitIdentity` → `set-global-git-identity` |
| `docker-login` / `docker-logout` / `show-docker-identity` | `Profile.Docker.psm1` → `docker.zsh` |
| `set-npm-token` | `Set-NpmToken` → `set-npm-token` |
| `diagnostics` (status report) | `Show-ProfileStatus` → `show-profile-status` |

Shared inputs already standardized across both repos: `~/.config/git/identities`,
`~/.docker/config.json`, `~/.npmrc`.

### Platform-specific (dispatch by OS, ~16)
Same intent, different mechanism (winget/PSGallery on Windows; apt/brew + GitHub-release
downloads on Linux/WSL). Examples drawn from `bootstrap.ps1` / `install.zsh`:

`install-git`, `install-gh`, `install-docker`, `install-nvm` (→ `install-node-lts`),
`install-python`, `install-rust`, `install-pnpm` (corepack), `install-zoxide`,
`install-fzf`, `install-bat` (+ config), `install-eza`, `install-git-delta`,
`install-lazygit`, `install-oh-my-posh`, `install-oh-my-zsh` (+ plugins; zsh only),
`install-obsidian`, `set-default-shell` (zsh only).

Each is **idempotent**: probe PATH / package list first, act only if absent; support `--dry-run`.

## windows-terminal-theme-adapter

A single **YAML theme** is the source of truth; the adapter transforms it into the target
terminal's native structure (Windows Terminal `settings.json` first). This fills the gap the
PowerShell repo leaves open (it has WT tab/pane helpers but no theme management).

Proposed theme schema (`theme.yml`):

```yaml
name: synthwave84
appearance:
  colorScheme: synthwave84        # name of the generated WT color scheme
  font: { face: "CaskaydiaCove Nerd Font", size: 11 }
  cursorShape: bar
  opacity: 90
  useAcrylic: true
  background: { image: "~/.config/starbase/bg.png", opacity: 0.3, stretch: uniformToFill }
palette:                           # 16 ANSI + special colors → WT color scheme entries
  background: "#262335"
  foreground: "#ffffff"
  black: "#262335"
  # … red/green/yellow/blue/magenta/cyan/white + bright variants …
  cursorColor: "#f97e72"
  selectionBackground: "#ffffff"
```

Adapter responsibilities:
- Map `palette.*` → a Windows Terminal color scheme object and upsert it into
  `settings.json` `schemes[]` (create-or-update by `name`, never clobber other schemes).
- Apply `appearance.*` to the relevant profile(s) (`defaults` or a named profile).
- Be idempotent and **back up `settings.json` before writing**.
- Keep the transform isolated so other terminals can get their own adapter later
  (the YAML stays the single source of truth).

## Open questions

- Where the canonical theme library lives (in-repo vs `~/.config/starbase/themes/`).
- macOS support timing (the cross-platform core already anticipates it; installers don't yet).
- Whether `starbase setup run` should be safe to wire into CI / fresh-machine bootstrap directly.
