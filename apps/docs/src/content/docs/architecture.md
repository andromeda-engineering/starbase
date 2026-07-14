---
title: Architecture Overview
description: How Starbase structures its hybrid monorepo across TypeScript and Rust.
---

Starbase uses a hybrid monorepo that combines two build systems working side by side.

## Monorepo Structure

### TypeScript — Turborepo

Turborepo manages the TypeScript packages, apps, and future front-end work:

- `apps/*` — deployable applications (including this docs site)
- `packages/*` — shared TypeScript libraries
- Task pipeline defined in `turbo.json`: `build`, `test`, `lint`, `typecheck`, `format`

### Rust — Cargo Workspace

Cargo manages the Rust crates, CLI binaries, and core logic:

- `crates/*` — Rust library crates
- `Cargo.toml` at the repo root defines the workspace members

## Design Principles

**Phase 0** intentionally avoids product implementation. The scaffold establishes:

1. Toolchain consistency (pinned Node, Rust toolchain via `rust-toolchain.toml`)
2. A single lock-file for JS dependencies (`pnpm-lock.yaml`)
3. ESLint + Prettier for TypeScript, `rustfmt` + `clippy` for Rust
4. CI-ready task graph so future packages slot in without reconfiguring pipelines

## Future Phases

Phase 1 introduces the `starbase` CLI binary (Rust, `crates/starbase`) with its first
subcommand, `doctor`. Later phases build on it:

- Additional `starbase` subcommands beyond `doctor` (e.g. `repo`, `setup`)
- SQLite persistence layer with SeaORM
- Agent configuration and local memory subsystem
- Repeatable workflow engine
