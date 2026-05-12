use crate::config::AppConfig;
use crate::geo;
use crate::http_tester::{HttpTester, TestConfig};
use crate::knife;
use crate::subs::Subscription;
use crate::utils;
use anyhow::{bail, Context, Result};
use rusqlite::Connection;
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
// use tempfile::NamedTempFile;

#[allow(dead_code)]
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
) -> Result<()> {
    let subs = crate::subs::load_subs(subs_path)?;
    let ids = if target == "all" {
        subs.iter().map(|s| s.id).collect()
    } else {
        utils::expand_ids(target)?
    };
    for id in ids {
        if let Some(sub) = subs.iter().find(|s| s.id == id) {
            update_single_sub(
                sub, protocol, limit, keep_raw, show_info, config, db, tester,
            )?;
        } else {
            eprintln!("⚠️ Подписка {} не найдена", id);
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn update_single_sub(
    sub: &Subscription,
    proto: &str,
    limit: usize,
    keep_raw: bool,
    show_info: bool,
    config: &AppConfig,
    db: Option<&Connection>,
    tester: &dyn HttpTester,
) -> Result<()> {
    println!("ℹ️ Обновление [{}] {}", sub.id, sub.name);
    let config_dir = dirs::config_dir()
        .context("Не удалось определить config директорию")?
        .join("vpn-manager");
    std::fs::create_dir_all(&config_dir)?;

    // 1. Загрузка подписки
    let raw_path = format!("/tmp/vpn-sub-{}-raw.txt", sub.id);
    if sub.url.starts_with("http://") || sub.url.starts_with("https://") {
        knife::fetch_subscription(&sub.url, Path::new(&raw_path))?;
    } else if sub.url.starts_with("file://") {
        fs::copy(sub.url.trim_start_matches("file://"), &raw_path)?;
    } else if Path::new(&sub.url).exists() {
        fs::copy(&sub.url, &raw_path)?;
    } else {
        bail!("Неизвестный источник: {}", sub.url);
    }
    let content = fs::read_to_string(&raw_path)?.replace('\r', "");
    fs::write(&raw_path, content)?;

    // 2. Фильтрация
    let filtered_path = format!("/tmp/vpn-sub-{}-filtered.txt", sub.id);
    utils::filter_subscription_file(&raw_path, &filtered_path, proto, limit)?;

    // 3. HTTP тесты (через объект-тестер)
    let active_urls = utils::get_active_urls(config);
    if active_urls.is_empty() {
        bail!("Нет активных тестовых URL");
    }

    let test_config = TestConfig {
        sub_id: sub.id,
        timeout: config.http_test_timeout,
        threads: config.http_test_threads,
        insecure: config.insecure,
        speedtest: config.speedtest,
        show_info,
        log_dir: PathBuf::from(&config.http_log_dir),
        config,
    };

    let results = tester.run_tests(Path::new(&filtered_path), &active_urls, &test_config)?;

    let live_files: Vec<PathBuf> = results
        .into_iter()
        .filter_map(|r| {
            if r.success {
                r.live_file_path
            } else {
                if let Some(err) = r.error {
                    eprintln!("⚠️ Ошибка HTTP теста для {}: {}", r.url, err);
                }
                None
            }
        })
        .collect();

    if live_files.is_empty() {
        bail!("Нет живых конфигов");
    }

    // 4. Сохранение статистики (если есть БД)
    if let Some(conn) = db {
        record_stats(sub.id, &active_urls, config, conn)?;
    }

    // 5. Слияние и сохранение
    let merged = format!("/tmp/vpn-sub-{}-live-merged.txt", sub.id);
    utils::merge_files(&live_files, &merged)?;
    let dest = config_dir.join(format!("sub_{}_live.txt", sub.id));
    fs::copy(&merged, &dest)?;
    let ts = chrono::Local::now().format("%Y-%m-%d %H:%M").to_string();
    fs::write(config_dir.join(format!("sub_{}_timestamp.txt", sub.id)), ts)?;

    // 6. Общий all_live и классификация
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
        let _ = fs::remove_file(&raw_path);
        let _ = fs::remove_file(&filtered_path);
        let _ = fs::remove_file(&merged);
        for f in &live_files {
            let _ = fs::remove_file(f);
        }
    }
    println!("✅ Подписка [{}] обновлена", sub.id);
    Ok(())
}

fn record_stats(
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

fn sanitize_filename(s: &str) -> String {
    s.replace(['/', ':', '?', '&'], "_")
}

fn classify_configs(input_path: &Path, config: &AppConfig) -> Result<()> {
    let config_dir = dirs::config_dir()
        .context("Не удалось определить config директорию")?
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
        "🌍 Классификация завершена: {} обработано, {} пропущено",
        total - skipped,
        skipped
    );
    Ok(())
}
