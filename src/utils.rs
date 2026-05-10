use anyhow::Result;
use std::fs;
// use std::io::{BufRead, BufReader};
use std::net::ToSocketAddrs;
// use std::path::Path;
use crate::config::AppConfig;

pub fn trim(s: &str) -> String {
    s.trim().to_string()
}

pub fn extract_host(link: &str) -> String {
    if link.starts_with("vless://") || link.starts_with("trojan://") || link.starts_with("ss://") {
        if let Some(at) = link.find('@') {
            let rest = &link[at+1..];
            let host = rest.split(|c: char| c == ':' || c == '/' || c == '?' || c == '#' || c == ' ').next().unwrap_or("unknown");
            return host.to_string();
        }
    } else if link.starts_with("vmess://") {
        let encoded = link.trim_start_matches("vmess://");
        if let Ok(decoded) = base64_decode(encoded) {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&decoded) {
                if let Some(add) = json.get("add").and_then(|v| v.as_str()) {
                    return add.to_string();
                }
            }
        }
    }
    "unknown".to_string()
}

fn base64_decode(s: &str) -> Result<String, base64::DecodeError> {
    use base64::{Engine as _, engine::general_purpose};
    let bytes = general_purpose::STANDARD.decode(s)?;
    Ok(String::from_utf8_lossy(&bytes).into())
}

pub fn resolve_ip(host: &str) -> Option<String> {
    (host, 0).to_socket_addrs().ok()?.next().map(|addr| addr.ip().to_string())
}

pub fn expand_ids(spec: &str) -> Result<Vec<usize>> {
    let mut ids = vec![];
    for part in spec.split(',') {
        let part = part.trim();
        if part.contains('-') {
            let bounds: Vec<&str> = part.split('-').collect();
            if bounds.len() != 2 { anyhow::bail!("неверный диапазон: {}", part); }
            let start: usize = bounds[0].parse()?;
            let end: usize = bounds[1].parse()?;
            ids.extend(start..=end);
        } else {
            ids.push(part.parse()?);
        }
    }
    Ok(ids)
}

pub fn filter_subscription_file(input: &str, output: &str, proto: &str, limit: usize) -> Result<()> {
    let content = fs::read_to_string(input)?;
    let lines: Vec<&str> = content.lines()
        .filter(|l| !l.trim().is_empty() && !l.trim().starts_with('#'))
        .collect();
    let filtered: Vec<&str> = if proto == "all" {
        lines
    } else {
        let prefix = format!("{}://", proto);
        lines.into_iter().filter(|l| l.starts_with(&prefix)).collect()
    };
    let limited = if limit > 0 && limit < filtered.len() {
        filtered[..limit].to_vec()
    } else { filtered };
    fs::write(output, limited.join("\n") + "\n")?;
    Ok(())
}

pub fn get_active_urls(config: &AppConfig) -> Vec<String> {
    let active_ids: Vec<&str> = config.http_url_active_ids.split(',').map(|s| s.trim()).collect();
    let mut urls = vec![];
    for entry in config.http_url_pool_data.split(';') {
        let parts: Vec<&str> = entry.splitn(2, '|').collect();
        if parts.len() == 2 {
            if active_ids.contains(&parts[0]) {
                urls.push(parts[1].to_string());
            }
        }
    }
    urls
}

pub fn merge_files(files: &[std::path::PathBuf], output: &str) -> Result<()> {
    let mut all = String::new();
    for f in files {
        all.push_str(&fs::read_to_string(f)?);
    }
    fs::write(output, unique_lines(&all))?;
    Ok(())
}

pub fn unique_lines(s: &str) -> String {
    let mut lines: Vec<&str> = s.lines().collect();
    lines.sort();
    lines.dedup();
    lines.join("\n") + "\n"
}

pub fn resolve_region(input: &str) -> Option<String> {
    match input {
        "ru" | "us" | "eu" | "de" | "pl" | "fi" | "nl" | "other" | "all" => Some(input.to_string()),
        "russia" => Some("ru".into()),
        "usa" => Some("us".into()),
        "europe" => Some("eu".into()),
        "germany" => Some("de".into()),
        "poland" => Some("pl".into()),
        "finland" => Some("fi".into()),
        "netherlands" => Some("nl".into()),
        _ => None,
    }
}
