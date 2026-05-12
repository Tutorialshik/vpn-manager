use crate::config::AppConfig;
use anyhow::{bail, Result};
use std::fs;
use std::net::ToSocketAddrs;

#[allow(dead_code)]
pub fn trim(s: &str) -> String {
    s.trim().to_string()
}

pub fn extract_host(link: &str) -> String {
    if link.starts_with("vless://") || link.starts_with("trojan://") || link.starts_with("ss://") {
        if let Some(at) = link.find('@') {
            let rest = &link[at + 1..];
            let host = rest
                .split([':', '/', '?', '#', ' '])
                .next()
                .unwrap_or("unknown");
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
    use base64::{engine::general_purpose, Engine as _};
    let bytes = general_purpose::STANDARD.decode(s)?;
    Ok(String::from_utf8_lossy(&bytes).into())
}

pub fn resolve_ip(host: &str) -> Option<String> {
    (host, 0)
        .to_socket_addrs()
        .ok()?
        .next()
        .map(|addr| addr.ip().to_string())
}

pub fn expand_ids(spec: &str) -> Result<Vec<usize>> {
    let mut ids = vec![];
    for part in spec.split(',') {
        let part = part.trim();
        if part.contains('-') {
            let bounds: Vec<&str> = part.split('-').collect();
            if bounds.len() != 2 {
                bail!("неверный диапазон: {}", part);
            }
            let start: usize = bounds[0].parse()?;
            let end: usize = bounds[1].parse()?;
            if start > end {
                bail!("начало диапазона больше конца: {}-{}", start, end);
            }
            ids.extend(start..=end);
        } else {
            ids.push(part.parse()?);
        }
    }
    Ok(ids)
}

pub fn filter_subscription_file(
    input: &str,
    output: &str,
    proto: &str,
    limit: usize,
) -> Result<()> {
    let content = fs::read_to_string(input)?;
    let lines: Vec<&str> = content
        .lines()
        .filter(|l| !l.trim().is_empty() && !l.trim().starts_with('#'))
        .collect();
    let filtered: Vec<&str> = if proto == "all" {
        lines
    } else {
        let prefix = format!("{}://", proto);
        lines
            .into_iter()
            .filter(|l| l.starts_with(&prefix))
            .collect()
    };
    let limited = if limit > 0 && limit < filtered.len() {
        filtered[..limit].to_vec()
    } else {
        filtered
    };
    fs::write(output, limited.join("\n") + "\n")?;
    Ok(())
}

pub fn get_active_urls(config: &AppConfig) -> Vec<String> {
    let active_ids: Vec<&str> = config
        .http_url_active_ids
        .split(',')
        .map(|s| s.trim())
        .collect();
    let mut urls = vec![];
    for entry in config.http_url_pool_data.split(';') {
        let parts: Vec<&str> = entry.splitn(2, '|').collect();
        if parts.len() == 2 && active_ids.contains(&parts[0]) {
            urls.push(parts[1].to_string());
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_expand_ids_single() {
        assert_eq!(expand_ids("5").unwrap(), vec![5]);
    }

    #[test]
    fn test_expand_ids_range() {
        assert_eq!(expand_ids("1-3").unwrap(), vec![1, 2, 3]);
    }

    #[test]
    fn test_expand_ids_mixed() {
        assert_eq!(expand_ids("1,3-5,7").unwrap(), vec![1, 3, 4, 5, 7]);
    }

    #[test]
    fn test_expand_ids_invalid_range_reversed() {
        assert!(expand_ids("3-1").is_err());
    }

    #[test]
    fn test_expand_ids_invalid_format() {
        assert!(expand_ids("1-3-5").is_err());
    }

    #[test]
    fn test_filter_subscription_file_all_proto() -> anyhow::Result<()> {
        let input = "vless://example.com\nvmess://example.com\ntrojan://example.com\n# comment\n\nss://example.com";
        let input_file = NamedTempFile::new()?;
        let output_file = NamedTempFile::new()?;
        fs::write(input_file.path(), input)?;

        filter_subscription_file(
            input_file.path().to_str().unwrap(),
            output_file.path().to_str().unwrap(),
            "all",
            0,
        )?;

        let result = fs::read_to_string(output_file.path())?;
        let expected =
            "vless://example.com\nvmess://example.com\ntrojan://example.com\nss://example.com\n";
        assert_eq!(result, expected);
        Ok(())
    }

    #[test]
    fn test_filter_subscription_file_specific_proto() -> anyhow::Result<()> {
        let input = "vless://a.com\nvmess://b.com\nvless://c.com";
        let input_file = NamedTempFile::new()?;
        let output_file = NamedTempFile::new()?;
        fs::write(input_file.path(), input)?;

        filter_subscription_file(
            input_file.path().to_str().unwrap(),
            output_file.path().to_str().unwrap(),
            "vless",
            0,
        )?;

        let result = fs::read_to_string(output_file.path())?;
        assert_eq!(result, "vless://a.com\nvless://c.com\n");
        Ok(())
    }

    #[test]
    fn test_filter_subscription_file_with_limit() -> anyhow::Result<()> {
        let input = "vless://a\nvless://b\nvless://c";
        let input_file = NamedTempFile::new()?;
        let output_file = NamedTempFile::new()?;
        fs::write(input_file.path(), input)?;

        filter_subscription_file(
            input_file.path().to_str().unwrap(),
            output_file.path().to_str().unwrap(),
            "vless",
            2,
        )?;

        let result = fs::read_to_string(output_file.path())?;
        // limit=2, должно остаться только первые две строки
        assert_eq!(result, "vless://a\nvless://b\n");
        Ok(())
    }

    #[test]
    fn test_resolve_region_short() {
        assert_eq!(resolve_region("ru"), Some("ru".into()));
        assert_eq!(resolve_region("eu"), Some("eu".into()));
    }

    #[test]
    fn test_resolve_region_full_name() {
        assert_eq!(resolve_region("russia"), Some("ru".into()));
        assert_eq!(resolve_region("germany"), Some("de".into()));
    }

    #[test]
    fn test_resolve_region_unknown() {
        assert_eq!(resolve_region("unknown"), None);
    }
}
