//! Minimal SQLite bootstrap for Phase 1 (schema ready for Phase 2 runs).

use std::path::Path;

use rusqlite::Connection;
use thiserror::Error;

/// Current schema version written on init.
pub const SCHEMA_VERSION: i32 = 1;

#[derive(Debug, Error)]
pub enum DbError {
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// Create parent dirs if needed and initialize `lumirix.sqlite`.
pub fn init_database(db_path: &Path) -> Result<(), DbError> {
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let conn = Connection::open(db_path)?;
    conn.execute_batch(
        r#"
        PRAGMA foreign_keys = ON;

        CREATE TABLE IF NOT EXISTS schema_version (
            version INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS runs (
            run_id TEXT PRIMARY KEY,
            started_at TEXT,
            ended_at TEXT,
            agent_command TEXT,
            exit_code INTEGER,
            base_commit TEXT,
            status TEXT
        );
        "#,
    )?;

    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM schema_version",
        [],
        |row| row.get(0),
    )?;

    if count == 0 {
        conn.execute(
            "INSERT INTO schema_version (version) VALUES (?1)",
            [SCHEMA_VERSION],
        )?;
    }

    Ok(())
}
