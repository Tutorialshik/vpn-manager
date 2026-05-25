use anyhow::{Context, Result};
use rusqlite::Connection;
use std::path::Path;

/// Открыть или создать БД и включить WAL
pub fn open_db(path: &Path) -> Result<Connection> {
    let conn = Connection::open(path).context("Не удалось открыть БД SQLite")?;
    conn.execute_batch("PRAGMA journal_mode=WAL;")?;
    create_tables(&conn)?;
    Ok(conn)
}

fn create_tables(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS profile_host_stats (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            host TEXT NOT NULL,
            profile_link_hash TEXT NOT NULL,
            success_count INTEGER NOT NULL DEFAULT 0,
            fail_count INTEGER NOT NULL DEFAULT 0,
            avg_latency_ms REAL,
            avg_bandwidth_mbps REAL,
            last_used INTEGER,
            score REAL,
            UNIQUE(host, profile_link_hash)
        );

        CREATE TABLE IF NOT EXISTS host_country_cache (
            host TEXT PRIMARY KEY,
            country TEXT NOT NULL,
            updated_at INTEGER NOT NULL
        );",
    )?;
    Ok(())
}
