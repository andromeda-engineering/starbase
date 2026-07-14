---
title: Schema Reference
description: SQLite v0 database schema and SeaORM entity definitions for Starbase.
---

This page is the human-readable source of truth for the Starbase v0 persistence schema.
The canonical DDL lives in `crates/starbase-db/migrations/0001_initial_schema.sql`; this doc
mirrors it and explains the design decisions.

## Design principles

- **SQLite v0, Postgres-portable.** The backend for local use is SQLite, but all column
  types (`TEXT`, `TIMESTAMP`) are valid in Postgres.  No SQLite-only pragmas or types
  appear in the DDL, so a future lift-and-shift to Postgres requires no schema changes.
- **UUIDs as `TEXT`.** SQLite has no native UUID type.  IDs are stored as plain text
  (UUID v4, canonical hyphenated form).  Postgres accepts `TEXT` as-is; a future Postgres
  migration can `ALTER COLUMN id TYPE UUID USING id::uuid`.
- **Timestamps as `TIMESTAMP`.** `CURRENT_TIMESTAMP` is ANSI SQL and works in both
  engines.  SeaORM maps this to `DateTimeUtc` in Rust via `chrono`.
- **Cascading deletes.** Child rows (`memory_entries`, `config_entries`) are deleted
  automatically when their parent `projects` row is removed.

---

## Tables

### `projects`

Tracks every repository / project registered with the `starbase` CLI.

| Column | Type | Nullable | Notes |
|--------|------|----------|-------|
| `id` | `TEXT` | NOT NULL | UUID v4 primary key |
| `name` | `TEXT` | NOT NULL | Human-readable project name |
| `root_path` | `TEXT` | NOT NULL | Absolute path to the project root on disk |
| `created_at` | `TIMESTAMP` | NOT NULL | Row creation time (default: `CURRENT_TIMESTAMP`) |
| `updated_at` | `TIMESTAMP` | NOT NULL | Last update time (default: `CURRENT_TIMESTAMP`) |

**Primary key:** `id`

**Relationships:**
- has many `memory_entries` (CASCADE DELETE)
- has many `config_entries` (CASCADE DELETE)

---

### `memory_entries`

Structured facts, decisions, or context notes attached to a project.  These are the
building blocks of agent context fed back to AI sessions.

| Column | Type | Nullable | Notes |
|--------|------|----------|-------|
| `id` | `TEXT` | NOT NULL | UUID v4 primary key |
| `project_id` | `TEXT` | NOT NULL | FK -> `projects.id` (CASCADE DELETE) |
| `kind` | `TEXT` | NOT NULL | Category: `"fact"`, `"decision"`, `"context"`, etc. |
| `key` | `TEXT` | NOT NULL | Short label / slug; unique within a project |
| `value` | `TEXT` | NOT NULL | Memory content -- free-form text or JSON string |
| `created_at` | `TIMESTAMP` | NOT NULL | Row creation time |
| `updated_at` | `TIMESTAMP` | NOT NULL | Last update time |

**Primary key:** `id`

**Unique constraint:** `(project_id, key)` -- one entry per key per project.

**Indexes:**
- `idx_memory_entries_project_id` on `(project_id)` -- fast lookup of all entries for a project
- `idx_memory_entries_kind` on `(kind)` -- filter by entry category

---

### `config_entries`

Per-project or global key-value configuration.  A `NULL` `project_id` means the entry
is global (not scoped to any single project).

| Column | Type | Nullable | Notes |
|--------|------|----------|-------|
| `id` | `TEXT` | NOT NULL | UUID v4 primary key |
| `project_id` | `TEXT` | NULL allowed | FK -> `projects.id`; `NULL` = global config |
| `key` | `TEXT` | NOT NULL | Configuration key |
| `value` | `TEXT` | NOT NULL | Configuration value (plain text or JSON string) |
| `created_at` | `TIMESTAMP` | NOT NULL | Row creation time |
| `updated_at` | `TIMESTAMP` | NOT NULL | Last update time |

**Primary key:** `id`

**Unique constraint:** `(project_id, key)` -- one value per key per scope (global or project).

**Index:**
- `idx_config_entries_project_id` on `(project_id)` -- fast lookup of all entries for a project

---

## SeaORM entities

Each table has a corresponding SeaORM entity in `crates/starbase-db/src/entities/`:

| Table | Entity module |
|-------|---------------|
| `projects` | `entities::project` |
| `memory_entries` | `entities::memory_entry` |
| `config_entries` | `entities::config_entry` |

Entities use `DateTimeUtc` (`chrono`) for timestamp columns and `Option<String>` for
nullable `project_id` in `config_entries`.

## Migrations

Migrations are embedded in the `starbase-db` crate via `sqlx::migrate!` and applied with
`starbase_db::run_migrations(db_url)`.  The single v0 migration file is:

```text
crates/starbase-db/migrations/0001_initial_schema.sql
```

Running migrations twice is safe -- all DDL uses `CREATE TABLE IF NOT EXISTS` and
`CREATE INDEX IF NOT EXISTS`.
