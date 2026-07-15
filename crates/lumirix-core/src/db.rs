//! SQLite bootstrap and run index helpers.

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

    let count: i64 =
        conn.query_row("SELECT COUNT(*) FROM schema_version", [], |row| row.get(0))?;

    if count == 0 {
        conn.execute(
            "INSERT INTO schema_version (version) VALUES (?1)",
            [SCHEMA_VERSION],
        )?;
    }

    Ok(())
}

/// Insert or replace a run row in the index.
#[allow(clippy::too_many_arguments)]
pub fn upsert_run(
    db_path: &Path,
    run_id: &str,
    started_at: &str,
    ended_at: Option<&str>,
    agent_command: &str,
    exit_code: Option<i32>,
    base_commit: Option<&str>,
    status: &str,
) -> Result<(), DbError> {
    // Ensure schema exists even if DB was partially created.
    init_database(db_path)?;
    let conn = Connection::open(db_path)?;
    conn.execute(
        r#"
        INSERT INTO runs (run_id, started_at, ended_at, agent_command, exit_code, base_commit, status)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
        ON CONFLICT(run_id) DO UPDATE SET
            started_at = excluded.started_at,
            ended_at = excluded.ended_at,
            agent_command = excluded.agent_command,
            exit_code = excluded.exit_code,
            base_commit = excluded.base_commit,
            status = excluded.status
        "#,
        rusqlite::params![
            run_id,
            started_at,
            ended_at,
            agent_command,
            exit_code,
            base_commit,
            status,
        ],
    )?;
    Ok(())
}
