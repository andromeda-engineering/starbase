-- Starbase v0 schema: project memory / config primitives
-- Designed to be Postgres-compatible (no SQLite-only types).
--
-- projects     – registered projects tracked by starbase
-- memory_entries – structured facts/notes attached to a project; feed agent context
-- config_entries  – per-project (or global) key-value configuration

CREATE TABLE IF NOT EXISTS projects (
    id          TEXT        NOT NULL PRIMARY KEY,   -- UUID v4 stored as text
    name        TEXT        NOT NULL,
    root_path   TEXT        NOT NULL,
    created_at  TIMESTAMP   NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at  TIMESTAMP   NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS memory_entries (
    id          TEXT        NOT NULL PRIMARY KEY,   -- UUID v4 stored as text
    project_id  TEXT        NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    kind        TEXT        NOT NULL,               -- e.g. "fact", "decision", "context"
    key         TEXT        NOT NULL,               -- short label / slug
    value       TEXT        NOT NULL,               -- the memory content
    created_at  TIMESTAMP   NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at  TIMESTAMP   NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (project_id, key)
);

CREATE TABLE IF NOT EXISTS config_entries (
    id          TEXT        NOT NULL PRIMARY KEY,   -- UUID v4 stored as text
    project_id  TEXT        REFERENCES projects(id) ON DELETE CASCADE,  -- NULL = global config
    key         TEXT        NOT NULL,
    value       TEXT        NOT NULL,
    created_at  TIMESTAMP   NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at  TIMESTAMP   NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (project_id, key)
);

-- Indexes for common access patterns
CREATE INDEX IF NOT EXISTS idx_memory_entries_project_id ON memory_entries (project_id);
CREATE INDEX IF NOT EXISTS idx_memory_entries_kind ON memory_entries (kind);
CREATE INDEX IF NOT EXISTS idx_config_entries_project_id ON config_entries (project_id);
