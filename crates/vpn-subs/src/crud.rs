use anyhow::{Context, Result};
use serde_json;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use url::Url;
use vpn_core::config::AppConfig;
use vpn_core::types::{Subscription, Subscriptions};
use vpn_core::utils;

pub fn load_subs(path: &Path) -> Result<Subscriptions> {
    if path.exists() {
        let content = fs::read_to_string(path)?;
        let v: serde_json::Value = serde_json::from_str(&content)?;
        if let Some(subs) = v.get("subscriptions").and_then(|s| s.as_array()) {
            Ok(serde_json::from_value(serde_json::Value::Array(
                subs.clone(),
            ))?)
        } else if v.is_array() {
            Ok(serde_json::from_value(v)?)
        } else {
            anyhow::bail!("Неверный формат subscriptions.json")
        }
    } else {
        Ok(vec![])
    }
}

pub fn save_subs(path: &Path, subs: &Subscriptions) -> Result<()> {
    let obj = serde_json::json!({ "subscriptions": subs });
    let json = serde_json::to_string_pretty(&obj)?;
    fs::write(path, json)?;
    Ok(())
}

fn shorten_url(url: &str) -> String {
    let parsed = Url::parse(url).ok();
    if let Some(parsed) = parsed {
        let domain = parsed.host_str().unwrap_or("");
        let path = parsed.path();
        let trimmed = if path.len() > 30 { &path[..30] } else { path };
        format!("{}/{}", domain, trimmed.trim_start_matches('/'))
    } else {
        if url.len() <= 55 {
            url.to_string()
        } else {
            format!("{}...{}", &url[..30], &url[url.len() - 20..])
        }
    }
}

fn is_https(url: &str) -> bool {
    url.starts_with("https://")
}

pub fn list_subscriptions(path: &Path, config: &AppConfig) {
    let subs = match load_subs(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Ошибка загрузки подписок: {e}");
            return;
        }
    };
    if subs.is_empty() {
        println!("Нет подписок"); // оставим без локализации пока, позже интегрируем
        return;
    }
    let config_dir = match dirs::config_dir() {
        Some(d) => d.join("vpn-manager"),
        None => {
            eprintln!("Не удалось определить конфигурационную директорию");
            return;
        }
    };
    println!("═══════════════════════ Подписки ═══════════════════════");
    println!(
        " {:<4} {:<20} {:>2} {:>6} {:>12} {:<30} {:>12}",
        "ID", "Имя", "TS", "Живых", "Обновлено", "Ссылка", "Автообн."
    );
    for sub in subs {
        let live_path = config_dir.join(format!("sub_{}_live.txt", sub.id));
        let count = if live_path.exists() {
            fs::read_to_string(&live_path)
                .map(|s| s.lines().count())
                .unwrap_or(0)
        } else {
            0
        };
        let ts_path = config_dir.join(format!("sub_{}_timestamp.txt", sub.id));
        let updated = fs::read_to_string(&ts_path).unwrap_or_else(|_| "—".into());
        let short_url = shorten_url(&sub.url);
        let insecure_mark = if is_https(&sub.url) { "🔒" } else { "🔓" };
        let auto_update = if config.auto_update_interval > 0
            && (config.auto_update_ids == "all"
                || utils::expand_ids(&config.auto_update_ids)
                    .unwrap_or_default()
                    .contains(&sub.id))
        {
            format!("{}м", config.auto_update_interval)
        } else {
            "—".to_string()
        };

        let color = if !sub.enabled || count == 0 {
            "\x1b[31m"
        } else {
            ""
        };
        println!(
            " {}{:<4} {:<20.20} {:<2} {:>6} {:>12} {:<30.30} {:>12}\x1b[0m",
            color,
            sub.id,
            sub.name,
            insecure_mark,
            count,
            updated.trim(),
            short_url,
            auto_update
        );
    }
}

pub fn add_subscription(path: &Path) -> Result<()> {
    let subs = load_subs(path).unwrap_or_default();
    let new_id = subs.iter().map(|s| s.id).max().unwrap_or(0) + 1;
    println!("Автоматический ID: {}", new_id);

    print!("URL подписки: ");
    io::stdout().flush()?;
    let mut url = String::new();
    io::stdin().read_line(&mut url)?;
    let url = url.trim().to_string();

    let name = if let Ok(resp) = reqwest::blocking::get(&url) {
        if let Ok(body) = resp.text() {
            body.lines()
                .find(|l| l.starts_with('#') && !l.starts_with("##"))
                .map(|l| l.trim_start_matches('#').trim().to_string())
                .or_else(|| {
                    Url::parse(&url)
                        .ok()
                        .and_then(|u| u.path_segments()?.next_back().map(|s| s.to_string()))
                        .filter(|s| !s.is_empty())
                })
                .unwrap_or_else(|| {
                    print!("Имя подписки: ");
                    io::stdout().flush().ok();
                    let mut name = String::new();
                    io::stdin().read_line(&mut name).ok();
                    name.trim().to_string()
                })
        } else {
            print!("Имя подписки: ");
            io::stdout().flush().ok();
            let mut name = String::new();
            io::stdin().read_line(&mut name).ok();
            name.trim().to_string()
        }
    } else {
        print!("Имя подписки: ");
        io::stdout().flush().ok();
        let mut name = String::new();
        io::stdin().read_line(&mut name).ok();
        name.trim().to_string()
    };

    let mut subs = subs;
    subs.push(Subscription {
        id: new_id,
        name,
        url,
        enabled: true,
    });
    subs.sort_by_key(|s| s.id);
    save_subs(path, &subs)?;
    println!("✅ Подписка добавлена");
    Ok(())
}

pub fn edit_subscription(path: &Path, id: usize) -> Result<()> {
    let mut subs = load_subs(path)?;
    let sub = subs
        .iter_mut()
        .find(|s| s.id == id)
        .context("Подписка не найдена")?;
    println!(
        "Текущие данные: ID={}, Имя={}, URL={}",
        sub.id, sub.name, sub.url
    );
    print!("Новое имя (Enter = оставить): ");
    io::stdout().flush()?;
    let mut name = String::new();
    io::stdin().read_line(&mut name)?;
    print!("Новый URL (Enter = оставить): ");
    io::stdout().flush()?;
    let mut url = String::new();
    io::stdin().read_line(&mut url)?;
    if !name.trim().is_empty() {
        sub.name = name.trim().into();
    }
    if !url.trim().is_empty() {
        sub.url = url.trim().into();
    }
    save_subs(path, &subs)?;
    println!("✅ Подписка изменена");
    Ok(())
}

pub fn remove_subscriptions(path: &Path, ids_spec: &str) -> Result<()> {
    let mut subs = load_subs(path)?;
    let ids = if ids_spec == "all" {
        (1..=subs.len()).collect()
    } else {
        utils::expand_ids(ids_spec)?
    };
    let mut removed = 0;
    subs.retain(|s| {
        if ids.contains(&s.id) {
            removed += 1;
            false
        } else {
            true
        }
    });
    for (i, s) in subs.iter_mut().enumerate() {
        s.id = i + 1;
    }
    save_subs(path, &subs)?;
    println!("✅ Удалено подписок: {}", removed);
    Ok(())
}

pub fn switch_subscriptions(path: &Path, ids_spec: &str, enable: bool) -> Result<()> {
    let mut subs = load_subs(path)?;
    let ids = if ids_spec == "all" {
        (1..=subs.len()).collect()
    } else {
        utils::expand_ids(ids_spec)?
    };
    for sub in subs.iter_mut() {
        if ids.contains(&sub.id) {
            sub.enabled = enable;
        }
    }
    save_subs(path, &subs)?;
    let action = if enable {
        "включены"
    } else {
        "выключены"
    };
    println!("✅ Подписки {}: {}", action, ids_spec);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::NamedTempFile;

    #[test]
    fn test_save_and_load_subs() -> anyhow::Result<()> {
        let file = NamedTempFile::new()?;
        let path = file.path();
        let subs = vec![
            Subscription {
                id: 1,
                name: "Test".into(),
                url: "https://test.com".into(),
                enabled: true,
            },
            Subscription {
                id: 2,
                name: "Other".into(),
                url: "vmess://example.com".into(),
                enabled: false,
            },
        ];
        save_subs(path, &subs)?;
        let loaded = load_subs(path)?;
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].name, "Test");
        assert_eq!(loaded[1].enabled, false);
        Ok(())
    }

    #[test]
    fn test_load_subs_empty_file() -> anyhow::Result<()> {
        let file = NamedTempFile::new()?;
        let path = file.path();
        fs::write(path, "[]")?;
        let subs = load_subs(path)?;
        assert!(subs.is_empty());
        Ok(())
    }

    #[test]
    fn test_load_subs_missing_file() -> anyhow::Result<()> {
        let path = std::path::Path::new("/tmp/non_existent_file_for_test.json");
        let subs = load_subs(path)?;
        assert!(subs.is_empty());
        Ok(())
    }

    #[test]
    fn test_remove_subscriptions() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("subs.json");
        let subs = vec![
            Subscription {
                id: 1,
                name: "s1".into(),
                url: "u1".into(),
                enabled: true,
            },
            Subscription {
                id: 2,
                name: "s2".into(),
                url: "u2".into(),
                enabled: true,
            },
        ];
        save_subs(&path, &subs)?;
        remove_subscriptions(&path, "1")?;
        let remaining = load_subs(&path)?;
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].id, 1); // после удаления id пересчитываются, бывший 2 стал 1
        Ok(())
    }

    #[test]
    fn test_switch_subscriptions() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("subs.json");
        let subs = vec![Subscription {
            id: 1,
            name: "s1".into(),
            url: "u1".into(),
            enabled: true,
        }];
        save_subs(&path, &subs)?;
        switch_subscriptions(&path, "1", false)?;
        let loaded = load_subs(&path)?;
        assert_eq!(loaded[0].enabled, false);
        Ok(())
    }
}
