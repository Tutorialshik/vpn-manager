use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use vpn_core::config::AppConfig;
use vpn_core::geo;
use vpn_core::utils;
use vpn_l10n as l10n;

/// Классификация конфигов по странам
pub fn classify_configs(input_path: &Path, config: &AppConfig) -> Result<()> {
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

    let mut regions: HashMap<String, Vec<String>> = HashMap::new();
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
        regions
            .entry("all".to_string())
            .or_default()
            .push(line.to_string());
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
