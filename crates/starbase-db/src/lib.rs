//! `starbase-db`: Starbase persistence layer.
//!
//! Provides:
//! - SeaORM entity definitions for the v0 schema (projects, memory_entries, config_entries)
//! - SQLx-backed migrations applied via [`run_migrations`]
//! - A helper to open a SeaORM [`DatabaseConnection`]
//!
//! The v0 backend is SQLite; the schema uses only Postgres-compatible types so that
//! a future migration to Postgres requires no DDL changes.

pub mod entities;

use anyhow::Result;
use sea_orm::{ConnectOptions, Database, DatabaseConnection};

/// Apply all pending migrations from the embedded `migrations/` directory.
///
/// `db_url` should be a valid SQLx SQLite URL, e.g. `"sqlite:///path/to/starbase.db"` or
/// `"sqlite::memory:"`.
pub async fn run_migrations(db_url: &str) -> Result<()> {
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .connect(db_url)
        .await?;
    sqlx::migrate!("./migrations").run(&pool).await?;
    pool.close().await;
    Ok(())
}

/// Open a SeaORM [`DatabaseConnection`] to the given SQLite database.
///
/// Migrations are **not** automatically run here; call [`run_migrations`] first.
pub async fn open(db_url: &str) -> Result<DatabaseConnection> {
    let opts = ConnectOptions::new(db_url.to_owned());
    let db = Database::connect(opts).await?;
    Ok(db)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use tempfile::TempDir;

    fn sqlite_url(path: &Path) -> String {
        format!("sqlite://{}?mode=rwc", path.display())
    }

    /// Runs migrations against a fresh temp-file SQLite database and checks that
    /// the expected tables exist in `sqlite_master`.
    #[tokio::test]
    async fn migrations_apply_and_tables_exist() {
        let dir = TempDir::new().expect("tempdir");
        let db_path = dir.path().join("starbase.db");
        let url = sqlite_url(&db_path);

        run_migrations(&url)
            .await
            .expect("migrations should apply cleanly");

        // Verify tables via raw sqlx query
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .connect(&url)
            .await
            .expect("connect");

        let tables: Vec<String> = sqlx::query_scalar(
            "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' AND name NOT LIKE '_sqlx_%' ORDER BY name",
        )
        .fetch_all(&pool)
        .await
        .expect("query sqlite_master");

        assert_eq!(
            tables,
            vec!["config_entries", "memory_entries", "projects"],
            "expected exactly the three v0 tables"
        );

        pool.close().await;
    }

    /// Ensures migrations are idempotent (safe to run twice on the same database).
    #[tokio::test]
    async fn migrations_are_idempotent() {
        let dir = TempDir::new().expect("tempdir");
        let db_path = dir.path().join("starbase.db");
        let url = sqlite_url(&db_path);

        run_migrations(&url).await.expect("first run");
        run_migrations(&url)
            .await
            .expect("second run should be a no-op");
    }
}
