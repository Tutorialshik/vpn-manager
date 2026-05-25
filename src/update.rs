use crate::geo;
use crate::l10n;
use anyhow::Context;
use chrono::Local;
use rusqlite::Connection;
use sha2::{Digest, Sha256};
use std::fs;
// use std::io::Write;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use vpn_core::config::AppConfig;
use vpn_core::http_tester::HttpTester;
use vpn_core::utils;
use vpn_subs::crud;
use vpn_subs::update as subs_update;

/// Координирует обновление одной или нескольких подписок, вызывает vpn-subs
#[allow(clippy::too_many_arguments)]
pub fn handle_update(
    target: &str,
    protocol: &str,
    limit: usize,
    keep_raw: bool,
    show_info: bool,
    config: &AppConfig,
    subs_path: &Path,
    db: Option<&Connection>,
    tester: &dyn HttpTester,
) -> anyhow::Result<()> {
    let subs = crud::load_subs(subs_path)?;
    let ids = if target == "all" {
        subs.iter().map(|s| s.id).collect()
    } else {
        utils::expand_ids(target)?
    };
    for id in ids {
        if let Some(sub) = subs.iter().find(|s| s.id == id) {
            update_single(
                sub, protocol, limit, keep_raw, show_info, config, db, tester,
            )?;
        } else {
            eprintln!("{}", l10n::t_fmt("subs.subs_not_found", &[&id.to_string()]));
        }
    }
    Ok(())
}

/// Обновляет одну подписку: загрузка, фильтрация, тесты, сохранение, классификация
#[allow(clippy::too_many_arguments)]
fn update_single(
    sub: &vpn_core::types::Subscription,
    proto: &str,
    limit: usize,
    keep_raw: bool,
    show_info: bool,
    config: &AppConfig,
    db: Option<&Connection>,
    tester: &dyn HttpTester,
) -> anyhow::Result<()> {
    println!(
        "{}",
        l10n::t_fmt("subs.update_started", &[&sub.id.to_string(), &sub.name])
    );
    let config_dir = dirs::config_dir()
        .context(l10n::t("proxy.config_dir_missing"))?
        .join("vpn-manager");

    // 1. Загрузка через vpn-subs (теперь внутри update_single_sub)
    let live_files =
        subs_update::update_single_sub(sub, proto, limit, keep_raw, show_info, config, tester)?;

    // 2. Сохранение результатов (слияние, запись в файлы)
    let merged = format!("/tmp/vpn-sub-{}-live-merged.txt", sub.id);
    utils::merge_files(&live_files, &merged)?;
    let dest = config_dir.join(format!("sub_{}_live.txt", sub.id));
    fs::copy(&merged, &dest)?;
    let ts = Local::now().format("%Y-%m-%d %H:%M").to_string();
    fs::write(config_dir.join(format!("sub_{}_timestamp.txt", sub.id)), ts)?;

    // 3. Статистика (если есть БД)
    if let Some(conn) = db {
        let active_urls = utils::get_active_urls(config);
        record_stats(sub.id, &active_urls, config, conn)?;
    }

    // 4. Обновление all_live и классификация
    let mut all_live_content = String::new();
    for entry in fs::read_dir(&config_dir)? {
        let entry = entry?;
        let fname = entry.file_name();
        if let Some(name) = fname.to_str() {
            if name.starts_with("sub_") && name.ends_with("_live.txt") {
                if let Ok(data) = fs::read_to_string(entry.path()) {
                    all_live_content.push_str(&data);
                }
            }
        }
    }
    let all_live = config_dir.join("all_live_merged.txt");
    fs::write(&all_live, utils::unique_lines(&all_live_content))?;
    classify_configs(&all_live, config)?;

    if !keep_raw {
        // Удаляем временные файлы (можно оставить, но пока уберём)
        let _ = fs::remove_file(format!("/tmp/vpn-sub-{}-raw.txt", sub.id));
        let _ = fs::remove_file(format!("/tmp/vpn-sub-{}-filtered.txt", sub.id));
        let _ = fs::remove_file(&merged);
        for f in &live_files {
            let _ = fs::remove_file(f);
        }
    }
    println!(
        "{}",
        l10n::t_fmt("subs.update_finished", &[&sub.id.to_string()])
    );
    Ok(())
}

/// Запись статистики в БД
fn record_stats(
    sub_id: usize,
    urls: &[String],
    config: &AppConfig,
    conn: &Connection,
) -> anyhow::Result<()> {
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
            if line.find("://").is_some() {
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
                    rusqlite::params![host, hash, latency.unwrap_or(999.0), bandwidth.unwrap_or(0.0), score],
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

pub(crate) fn sanitize_filename(s: &str) -> String {
    s.replace(['/', ':', '?', '&'], "_")
}

/// Классификация конфигов по странам
fn classify_configs(input_path: &Path, config: &AppConfig) -> anyhow::Result<()> {
    let config_dir = dirs::config_dir()
        .context(l10n::t("proxy.config_dir_missing"))?
        .join("vpn-manager");
    let lists_dir = config_dir.join("lists");
    fs::create_dir_all(&lists_dir)?;
    let eu_countries = "AT BE BG HR CY CZ DK EE FI FR DE GR HU IE IT LV LT LU MT NL PL PT RO SK SI ES SE CH GB IS NO";
    let eu_list: Vec<&str> = eu_countries.split_whitespace().collect();

    let content = fs::read_to_string(input_path)?;
    let lines: Vec<&str> = content.lines().collect();
    let total = lines.len();
    let mut regions: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    regions.insert("all".into(), vec![]);
    for region in &["ru", "us", "eu", "de", "pl", "fi", "nl", "other"] {
        regions.insert(region.to_string(), vec![]);
    }

    let mut skipped = 0;
    for line in &lines {
        if line.trim().is_empty() {
            continue;
        }
        let host = utils::extract_host(line);
        if host == "unknown" {
            skipped += 1;
            continue;
        }
        let ip = utils::resolve_ip(&host);
        if ip.is_none() {
            skipped += 1;
            continue;
        }
        let ip = ip.unwrap();
        let code = geo::country_code(&ip, &config.geoip_db);
        let region = match code.as_deref() {
            Some("RU") => "ru",
            Some("US") => "us",
            Some("DE") => "de",
            Some("PL") => "pl",
            Some("FI") => "fi",
            Some("NL") => "nl",
            Some(c) if eu_list.contains(&c) => "eu",
            _ => "other",
        };
        if let Some(v) = regions.get_mut(region) {
            v.push(line.to_string());
        }
        regions.get_mut("all").unwrap().push(line.to_string());
    }

    for (region, lines) in regions.iter_mut() {
        lines.sort();
        lines.dedup();
        let file_path = lists_dir.join(format!("{}.txt", region));
        fs::write(file_path, lines.join("\n") + "\n")?;
    }
    println!(
        "{}",
        l10n::t_fmt(
            "subs.classify_finished",
            &[&(total - skipped).to_string(), &skipped.to_string()]
        )
    );
    Ok(())
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
        // Используем ту же sanitize_filename, что и record_stats
        let sanitized = sanitize_filename("https://test.com");
        let log_file = log_dir.join(format!("vpn-http-1-{}.log", sanitized));
        std::fs::write(
            &log_file,
            "https://test.com Delay: 150ms Speed: 50.0 Mbps\n",
        )?;

        let mut config = AppConfig::default();
        config.http_log_dir = log_dir.to_str().unwrap().to_string();

        let conn = setup_db();
        let urls = vec!["https://test.com".to_string()];

        record_stats(1, &urls, &config, &conn)?;

        let count: i64 = conn.query_row("SELECT COUNT(*) FROM profile_host_stats", [], |row| {
            row.get(0)
        })?;
        assert_eq!(count, 1);

        Ok(())
    }
}
