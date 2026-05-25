use anyhow::Result;
use rusqlite::Connection;
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use vpn_core::config::AppConfig;

use super::utils::sanitize_filename;

/// Запись статистики в БД
pub fn record_stats(
    sub_id: usize,
    urls: &[String],
    config: &AppConfig,
    conn: &Connection,
) -> Result<()> {
    let log_dir = PathBuf::from(&config.http_log_dir);
    let tx = conn.unchecked_transaction()?;

    for url in urls {
        let log_path = log_dir.join(format!(
            "vpn-http-{}-{}.log",
            sub_id,
            sanitize_filename(url)
        ));
        if !log_path.exists() {
            continue;
        }
        let file = fs::File::open(&log_path)?;
        let reader = BufReader::new(file);

        for line in reader.lines() {
            let line = match line {
                Ok(l) => l,
                Err(_) => continue,
            };
            if line.contains("://") {
                let link = line[..line.find(' ').unwrap_or(line.len())].to_string();
                let hash = format!("{:x}", Sha256::digest(link.as_bytes()));
                let host = url::Url::parse(url)
                    .map(|u| u.host_str().unwrap_or("unknown").to_string())
                    .unwrap_or_else(|_| "unknown".to_string());

                let latency = extract_latency(&line);
                let bandwidth = extract_bandwidth(&line);
                let score = calculate_score(latency, bandwidth, &config.parallel_strategy);

                tx.execute(
                    "INSERT INTO profile_host_stats (host, profile_link_hash, success_count, avg_latency_ms, avg_bandwidth_mbps, score, last_used)
                     VALUES (?1, ?2, 1, ?3, ?4, ?5, strftime('%s','now'))
                     ON CONFLICT(host, profile_link_hash) DO UPDATE SET
                     success_count = success_count + 1,
                     avg_latency_ms = (avg_latency_ms * success_count + ?3) / (success_count + 1),
                     avg_bandwidth_mbps = (avg_bandwidth_mbps * success_count + ?4) / (success_count + 1),
                     score = ?5,
                     last_used = strftime('%s','now')",
                    rusqlite::params![
                        host,
                        hash,
                        latency.unwrap_or(999.0),
                        bandwidth.unwrap_or(0.0),
                        score
                    ],
                )?;
            }
        }
    }

    tx.commit()?;
    Ok(())
}

fn extract_latency(line: &str) -> Option<f64> {
    let re = regex::Regex::new(r"Delay: (\d+)ms").ok()?;
    re.captures(line)?.get(1)?.as_str().parse::<f64>().ok()
}

fn extract_bandwidth(line: &str) -> Option<f64> {
    let re = regex::Regex::new(r"Speed: ([\d.]+) Mbps").ok()?;
    re.captures(line)?.get(1)?.as_str().parse::<f64>().ok()
}

fn calculate_score(latency: Option<f64>, bandwidth: Option<f64>, strategy: &str) -> f64 {
    match strategy {
        "latency" => {
            if let Some(lat) = latency {
                1000.0 / lat
            } else {
                0.0
            }
        }
        "bandwidth" => bandwidth.unwrap_or(0.0),
        _ => 1.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use rusqlite::Connection;
    use vpn_core::config::AppConfig;

    fn setup_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
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
            );",
        )
        .unwrap();
        conn
    }

    #[test]
    fn test_record_stats() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let log_dir = dir.path();
        let test_url = "https://test.com";
        let filename = format!("vpn-http-1-{}.log", sanitize_filename(test_url));
        let log_file = log_dir.join(&filename);
        std::fs::write(
            &log_file,
            "https://test.com 200 Delay: 150ms Speed: 50.0 Mbps\n",
        )?;

        let mut config = AppConfig::default();
        config.http_log_dir = log_dir.to_string_lossy().into();

        let conn = setup_db();
        let urls = vec![test_url.to_string()];

        record_stats(1, &urls, &config, &conn)?;

        let count: i64 = conn.query_row("SELECT COUNT(*) FROM profile_host_stats", [], |row| {
            row.get(0)
        })?;
        assert_eq!(count, 1, "record_stats не добавила запись");

        Ok(())
    }
}
