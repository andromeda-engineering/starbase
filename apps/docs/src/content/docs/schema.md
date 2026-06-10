---
title: Schema Reference
description: SQLite database schema and SeaORM entity definitions for NeonOS.
---

:::caution[Coming Soon]
The NeonOS persistence layer is planned for a future phase. This page is a placeholder and will be updated when the SQLite/SeaORM schema is implemented.
:::

## Overview

NeonOS will use SQLite as its local persistence store, managed through [SeaORM](https://www.sea-ql.org/SeaORM/) — an async, dynamic ORM for Rust.

## Planned Entity Areas

### Agent Configuration

Stores agent definitions, capability sets, and per-agent settings.

### Local Memory

Persists conversation context, factual notes, and project-scoped memory entries across sessions.

### Workflow Registry

Tracks registered workflows, their steps, and execution history.

### Repository Metadata

Caches repository state, toolchain info, and computed scaffolding results.

## Migration Strategy

Schema migrations will be managed with SeaORM's migration crate, giving each schema change a versioned, reversible migration file. These will live in a planned `crates/migration/` crate (not yet present — the workspace currently contains `crates/neon` and `crates/stub-crate`).
