# ADR 0004: Interactive CLI via `clap` + `interactive-clap` + `inquire` (Rust), not a Node/commander.js tool

## Status

Accepted

## Context

`neon repo init` (NEO-41) and `neon setup` require an interactive, wizard-style UX:
prompting for missing inputs, presenting menus, and varying subsequent questions based on
earlier answers (the pattern often called "interactive commander" in the Node ecosystem).
commander.js was surveyed as a UX reference for this pattern.

NeonOS is **Rust-first** and already ships a single `neon` binary backed by `clap`
(`neon repo init` was implemented in Rust in NEO-41). A separate Node/commander.js tool
was considered for the interactive layer. At the same time, `neon repo init` must support
a repo-type matrix (NEO-48: visibility × license × languages) and an interactive wizard
flow (NEO-49), and `neon setup` needs the same interactive core.

## Decision

Implement the interactive layer entirely in Rust, inside the existing `neon` binary:

- **`clap`** — the existing command-line parser; provides the top-level subcommand
  structure and all non-interactive (fully-flagged) paths.
- **`interactive-clap`** derive macro — wraps a `clap` struct so that any missing
  required argument triggers an automatic prompt rather than an error. This gives a
  zero-friction interactive path with no duplicated argument definitions.
- **`inquire`** — called inside command handlers for wizard steps where the next question
  depends on a prior answer (e.g. prompt for `license` only when `visibility = public`;
  `neon repo init` wizard — NEO-49). `inquire` handles dependent-prompt flows that
  `interactive-clap` alone cannot model.
- **Non-interactive (CI) path** — all prompts are bypassed when every required flag is
  supplied, or when `--yes` is passed. This is the scripting / CI contract; it must never
  regress.

commander.js remains a **UX reference only** — the interaction patterns it documents informed
the design, but no Node tooling ships.

## Consequences

- **One binary, no CLI split** — `neon repo init` and `neon setup` share a single
  Rust binary. The already-shipped NEO-41 `repo init` extends directly; no parallel
  Node entrypoint to maintain or reconcile.
- **New dependencies** — `interactive-clap` and `inquire` are added as `[dependencies]`
  in the relevant crate(s). Both are pure Rust and have no Node/npm surface.
- **Reusable interactive core** — the same `interactive-clap` + `inquire` pattern is
  usable across all `neon` subcommands that need wizard flows (`neon setup`, future
  commands).
- **Rejected alternative: separate Node/commander.js tool** — would split the CLI in two,
  orphan the Rust `neon repo init` already in production (NEO-41), and require users to
  have a Node runtime in every environment where `neon` runs. Rejected on Rust-first
  principle and integration cost.
- **commander.js** — referenced for UX patterns only; not a runtime dependency.
