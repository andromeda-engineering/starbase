---
title: starbase CLI Reference
description: Command reference for the starbase command-line interface.
---

The `starbase` CLI is the primary interface for interacting with Starbase from your terminal.

:::note
The CLI is under active development. Commands documented here reflect the current Phase 1 interface.
:::

## Global Flags

| Flag | Short | Description |
|------|-------|-------------|
| `--help` | `-h` | Print help for any command |
| `--version` | `-V` | Print the current `starbase` version |

---

## `starbase`

Prints the top-level help and lists available subcommands.

```sh
starbase
```

**Output example:**

```text
Starbase CLI – developer environment diagnostics and tooling

Usage: starbase <COMMAND>

Commands:
  doctor  Gather and display environment diagnostics
  help    Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```

---

## `starbase doctor`

A **read-only** health dashboard. It probes your local environment and reports detected
tool versions, your Git identity, and repository health. It never mutates anything.

```sh
starbase doctor
```

**What it reports:**

- **Tooling** — detected versions of `git`, `rustc`, `cargo`, `node`, `pnpm`, and `docker`
  (a missing tool is shown as `not found`).
- **Git Identity** — the effective `user.name` / `user.email`.
- **Repo Health** — current branch, short `HEAD`, and the count of dirty (uncommitted) files.

When run in an interactive terminal, `doctor` opens a multi-pane TUI (quit with `q` or `Esc`).
When stdout is **not** a TTY (piped or in CI), it prints the same information as plain text —
useful for logging and scripts:

```sh
starbase doctor | cat        # bash / zsh
starbase doctor | Out-String # PowerShell
```

**Plain-text output example:**

```text
=== Tooling ===
     git: git version 2.47.0
   rustc: rustc 1.96.0
   cargo: cargo 1.96.0
    node: v22.14.0
    pnpm: 9.15.0
  docker: Docker version 27.4.0

=== Git Identity ===
  name:  Ada Lovelace
  email: ada@example.com

=== Repo Health ===
  branch:    main
  HEAD:      a1b2c3d
  dirty:     0 file(s)
```

`starbase doctor` is diagnostic and read-only: it always exits `0`, reporting missing tools as
`not found` rather than failing. (It exits non-zero only on a genuine terminal I/O error.)
