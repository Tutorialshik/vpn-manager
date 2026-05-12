use anyhow::{Context, Result};
use rusqlite::{params, Connection};
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

#[allow(dead_code)]
pub fn cache_host_country(conn: &Connection, host: &str, country: &str) -> Result<()> {
    let now = chrono::Utc::now().timestamp();
    conn.execute(
        "INSERT OR REPLACE INTO host_country_cache (host, country, updated_at) VALUES (?1, ?2, ?3)",
        params![host, country, now],
    )?;
    Ok(())
}

#[allow(dead_code)]
pub fn get_cached_country(conn: &Connection, host: &str, ttl: u64) -> Option<String> {
    let now = chrono::Utc::now().timestamp();
    let mut stmt = conn
        .prepare("SELECT country, updated_at FROM host_country_cache WHERE host = ?1")
        .ok()?;
    let row = stmt
        .query_row(params![host], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })
        .ok()?;
    let (country, updated_at) = row;
    if (now - updated_at) < ttl as i64 {
        Some(country)
    } else {
        None
    }
}
