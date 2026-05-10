use anyhow::{Context, Result};
use reqwest::blocking::Client;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use tempfile::NamedTempFile;
use crate::config::AppConfig;
use crate::subs::{load_subs, Subscription};
use crate::utils;

pub fn handle_update(
    target: &str,
    protocol: &str,
    limit: usize,
    keep_raw: bool,
    config: &AppConfig,
    subs_path: &Path,
) -> Result<()> {
    let subs = load_subs(subs_path)?;
    let enabled_subs: Vec<_> = subs.into_iter().filter(|s| s.enabled).collect();

    if target == "all" {
        for sub in enabled_subs {
            update_single_sub(&sub, protocol, limit, keep_raw, config)?;
        }
    } else {
        let ids = utils::expand_ids(target)?;
        for id in ids {
            if let Some(sub) = enabled_subs.iter().find(|s| s.id == id) {
                update_single_sub(sub, protocol, limit, keep_raw, config)?;
            } else {
                eprintln!("⚠️ Подписка {} не найдена", id);
            }
        }
    }
    Ok(())
}

fn update_single_sub(
    sub: &Subscription,
    proto: &str,
    limit: usize,
    keep_raw: bool,
    config: &AppConfig,
) -> Result<()> {
    println!("ℹ️ Обновление [{}] {}", sub.id, sub.name);
    let config_dir = dirs::config_dir().unwrap().join("vpn-manager");
    std::fs::create_dir_all(&config_dir)?;

    // 1. Загрузка подписки
    let raw_path = format!("/tmp/vpn-sub-{}-raw.txt", sub.id);
    if sub.url.starts_with("http://") || sub.url.starts_with("https://") {
        let client = Client::new();
        let resp = client.get(&sub.url).send()?;
        let bytes = resp.bytes()?;
        fs::write(&raw_path, bytes)?;
    } else if sub.url.starts_with("file://") {
        let file_path = sub.url.trim_start_matches("file://");
        fs::copy(file_path, &raw_path)?;
    } else if Path::new(&sub.url).exists() {
        fs::copy(&sub.url, &raw_path)?;
    } else {
        anyhow::bail!("Неизвестный источник: {}", sub.url);
    }

    // 2. Удаление \r
    let content = fs::read_to_string(&raw_path)?;
    let content = content.replace('\r', "");
    fs::write(&raw_path, content)?;

    // 3. Фильтрация
    let filtered_path = format!("/tmp/vpn-sub-{}-filtered.txt", sub.id);
    utils::filter_subscription_file(&raw_path, &filtered_path, proto, limit)?;

    // 4. HTTP тесты
    let active_urls = utils::get_active_urls(config);
    if active_urls.is_empty() {
        anyhow::bail!("Нет активных тестовых URL");
    }

    let live_files = run_http_tests(sub.id, &filtered_path, &active_urls, config)?;
    if live_files.is_empty() {
        anyhow::bail!("Нет живых конфигов");
    }

    // 5. Слияние и сохранение
    let merged = format!("/tmp/vpn-sub-{}-live-merged.txt", sub.id);
    utils::merge_files(&live_files, &merged)?;
    let dest = config_dir.join(format!("sub_{}_live.txt", sub.id));
    fs::copy(&merged, &dest)?;

    // 6. Timestamp
    let ts = chrono::Local::now().format("%Y-%m-%d %H:%M").to_string();
    fs::write(config_dir.join(format!("sub_{}_timestamp.txt", sub.id)), ts)?;

    // 7. Общий all_live и классификация
    let mut all_live_content = String::new();
    for entry in fs::read_dir(&config_dir)? {
        let entry = entry?;
        let fname = entry.file_name();
        if let Some(name) = fname.to_str() {
            if name.starts_with("sub_") && name.ends_with("_live.txt") {
                let data = fs::read_to_string(entry.path()).unwrap_or_default();
                all_live_content.push_str(&data);
            }
        }
    }
    let all_live = config_dir.join("all_live_merged.txt");
    fs::write(&all_live, utils::unique_lines(&all_live_content))?;
    classify_configs(&all_live, config)?;

    // 8. Очистка если не keep_raw
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

fn run_http_tests(sub_id: usize, config_file: &str, urls: &[String], config: &AppConfig) -> Result<Vec<PathBuf>> {
    let timeout = config.http_test_timeout;
    let threads = config.http_test_threads;
    let insecure = config.insecure;
    let speedtest = config.speedtest;
    let log_dir = PathBuf::from(&config.http_log_dir);
    fs::create_dir_all(&log_dir)?;

    let mut handles = vec![];
    let config_file = config_file.to_owned();
    for url in urls {
        let url = url.clone();
        let config_f = config_file.clone();
        let log = log_dir.join(format!("vpn-http-{}-{}.log", sub_id, sanitize_filename(&url)));
        let live_file = NamedTempFile::new()?.into_temp_path(); // будет существовать до конца
        let live_path = live_file.to_path_buf();
        let handle = std::thread::spawn(move || -> Result<PathBuf> {
            let mut cmd = Command::new("xray-knife");
            cmd.arg("http")
                .arg("-f").arg(&config_f)
                .arg("-d").arg(timeout.to_string())
                .arg("-t").arg(threads.to_string())
                .arg("-u").arg(&url)
                .arg("-o").arg(&live_path);
            if insecure { cmd.arg("-e"); }
            if speedtest { cmd.arg("--speedtest"); }
            let output = cmd.output()?;
            // записываем логи
            if let Ok(mut f) = fs::File::create(&log) {
                f.write_all(&output.stdout).ok();
                f.write_all(&output.stderr).ok();
            }
            if Path::new(&live_path).exists() && fs::metadata(&live_path)?.len() > 0 {
                // keep tempfile alive by persisting it? We'll move to permanent /tmp location
                let perm_path = std::path::PathBuf::from(format!("/tmp/vpn-sub-{}-live-{}.txt", sub_id, sanitize_filename(&url)));
                fs::rename(&live_path, &perm_path)?;
                Ok(perm_path)
            } else {
                anyhow::bail!("empty live result")
            }
        });
        handles.push(handle);
    }

    let mut live_paths = vec![];
    for h in handles {
        match h.join().unwrap() {
            Ok(p) => live_paths.push(p),
            Err(e) => eprintln!("⚠️ Ошибка HTTP теста: {e}"),
        }
    }
    Ok(live_paths)
}

fn sanitize_filename(s: &str) -> String {
    s.replace('/', "_").replace(':', "_").replace('?', "_").replace('&', "_")
}

fn classify_configs(input_path: &Path, config: &AppConfig) -> Result<()> {
    let lists_dir = dirs::config_dir().unwrap().join("vpn-manager").join("lists");
    fs::create_dir_all(&lists_dir)?;
    let eu_countries = "AT BE BG HR CY CZ DK EE FI FR DE GR HU IE IT LV LT LU MT NL PL PT RO SK SI ES SE CH GB IS NO";
    let eu_list: Vec<&str> = eu_countries.split_whitespace().collect();

    let content = fs::read_to_string(input_path)?;
    let lines: Vec<&str> = content.lines().collect();
    let total = lines.len();
    let mut regions: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
    regions.insert("all".into(), vec![]);
    for region in &["ru", "us", "eu", "de", "pl", "fi", "nl", "other"] {
        regions.insert(region.to_string(), vec![]);
    }

    let mut skipped = 0;
    for line in &lines {
        if line.trim().is_empty() { continue; }
        let host = utils::extract_host(line);
        if host == "unknown" { skipped += 1; continue; }
        let ip = utils::resolve_ip(&host);
        if ip.is_none() { skipped += 1; continue; }
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

    // объединяем eu страны в eu (уже сделано)
    println!("🌍 Классификация завершена: {} обработано, {} пропущено", total - skipped, skipped);
    Ok(())
}
