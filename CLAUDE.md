# NeonOS — project instructions

NeonOS is an opinionated, open-source, **Rust-first AI-native developer environment**
exposed as the `neon` CLI: repo setup, local tooling, agent configuration, project
memory, and repeatable development workflows. Repo: `MyNameReallySux/neonos`.

> These instructions are authoritative for work in this repo. The user's global
> conventions (`~/.claude/CLAUDE.md`) still apply — especially the Windows/PowerShell
> environment rules and the "never use bash heredocs for commit/PR text" rule.

## Stack & layout

Hybrid monorepo (see `docs/adr/0001-use-hybrid-turborepo-cargo-workspace.md`):

- **Rust** (Cargo workspace) — the `neon` CLI and core logic. Crates live in `crates/`.
  The CLI binary crate is `crates/neon`. Toolchain pinned by `rust-toolchain.toml`.
- **TypeScript/JS** (Turborepo + pnpm) — apps and packages. Apps in `apps/`
  (incl. the Astro Starlight docs site at `apps/docs`), shared packages in `packages/`.
- Root config: `Cargo.toml`, `pnpm-workspace.toml`, `turbo.json`, `tsconfig.base.json`.

Phase docs in `docs/product/`, decisions in `docs/adr/`, architecture in
`docs/architecture/`. Read the relevant ones before changing a surface they describe.

## Conventions

- **Commits:** [Conventional Commits](https://www.conventionalcommits.org)
  (`feat:`, `fix:`, `docs:`, `chore:`, `refactor:`, `test:`, …). On Windows, write the
  message to a temp file and `git commit -F <file>` — **never** bash heredocs.
- **Branches:** follow the active branch-pattern. Linear suggests a branch name per issue
  (e.g. `chris/neo-1-...`); prefer it so the PR auto-links the ticket.
- **PRs:** one logical unit per PR; conventional title; body has a summary + test plan and
  links the Linear issue. Use the `pr` / `ship-it` skills. Open as **draft**; a human marks
  it ready (`ship-ready`) to trigger CI and merge. CI must be green before merge.
- **Post-merge:** clean up worktrees/branches with the `pr-merge` skill.
- **Style:** match surrounding code. Rust is `rustfmt` + `clippy` clean
  (`cargo fmt`, `cargo clippy`); TS is eslint-clean. No bulk auto-fixers across packages,
  no unrelated refactors.

## Parallel execution discipline

- **Worktrees.** Parallel work that touches **overlapping files** runs in separate git
  worktrees (one branch each). Purely **additive work to disjoint directories** (a new
  crate/package) may share one feature worktree. The Engineering Manager states which
  applies per slice and ensures cleanup when done.
- **Ports.** Any server gets a **unique** port; never share. Reuse an already-running
  server before spawning a new one (validator asks a runner first).

## Linear task lifecycle

Project **NeonOS** (`38e4d31c-021e-4825-9d21-fd0bdc42aa35`), team **NeonWave** (key `NEO`).
**Every unit of work has a ticket.** No implementation proceeds untracked.

State machine (use these exact NEO states):

| State | When to set it | Who |
|---|---|---|
| **Backlog** | Issue created, not yet scheduled into a phase. | anyone filing |
| **Todo** | Accepted into the current phase / ready to be picked up. | Product Owner / EM |
| **In Progress** | The moment work starts — implementer assigned and the branch/worktree is created. | Engineering Manager |
| **In Review** | The PR is opened (draft is fine). **Never skip this state.** | whoever opens the PR |
| **Done** | **Only** after the PR is merged to `main` **and** the Validator has passed. | Project Manager on merge |
| **Canceled** | Work dropped. Product Owner records why. | Product Owner |
| **Duplicate** | Superseded by another issue; link it. | anyone |

Rules of thumb:
- One ticket per isolated unit of work (ideally one per package/crate).
- Move to **In Progress** *as work begins*, not retroactively. Move to **Done** *after*
  merge + validation, not at PR open.
- The **Project Manager** gates merges on conventions; the **Product Owner** gates on plan
  alignment (and updates docs/tickets when reality diverges from the plan).

## Roles (project-local agents in `.claude/agents/`)

- **project-manager** (opus) — convention & ticket-lifecycle gatekeeper; gates merges.
- **product-owner** (opus) — plan/scope/ADR alignment; reconciles drift into the docs.
- **engineering-manager** (fable) — plans slices, decides the agent breakdown, owns
  worktree/port/cleanup discipline. **Also the advisor** (stronger-reviewer take on approach).
- **validator** (sonnet) — runs the built thing and verifies behavior; reuses running servers.

Generic ICs (researcher, implementer, runner, debugger, …) come from the global fleet.
