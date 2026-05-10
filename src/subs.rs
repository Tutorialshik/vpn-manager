use anyhow::{Context, Result};
use std::io::{self, Write};
use std::path::Path;
use crate::config::AppConfig;
use crate::update;
use crate::utils;
use crate::SubsCmd;
use crate::SwitchCmd;

use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Subscription {
    pub id: usize,
    pub name: String,
    pub url: String,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_enabled() -> bool { true }

type Subscriptions = Vec<Subscription>;

pub fn load_subs(path: &Path) -> Result<Subscriptions> {
    if path.exists() {
        let content = fs::read_to_string(path)?;
        let v: serde_json::Value = serde_json::from_str(&content)?;
        if let Some(subs) = v.get("subscriptions").and_then(|s| s.as_array()) {
            Ok(serde_json::from_value(serde_json::Value::Array(subs.clone()))?)
        } else if v.is_array() {
            Ok(serde_json::from_value(v)?)
        } else {
            anyhow::bail!("Неверный формат subscriptions.json")
        }
    } else {
        Ok(vec![])
    }
}

fn save_subs(path: &Path, subs: &Subscriptions) -> Result<()> {
    let obj = serde_json::json!({ "subscriptions": subs });
    let json = serde_json::to_string_pretty(&obj)?;
    fs::write(path, json)?;
    Ok(())
}

fn shorten_url(url: &str) -> String {
    let parsed = url::Url::parse(url).ok();
    if let Some(parsed) = parsed {
        let domain = parsed.host_str().unwrap_or("");
        let path = parsed.path();
        let trimmed = if path.len() > 30 { &path[..30] } else { path };
        format!("{}/{}", domain, trimmed.trim_start_matches('/'))
    } else {
        if url.len() <= 55 { url.to_string() } else { format!("{}...{}", &url[..30], &url[url.len()-20..]) }
    }
}

fn is_https(url: &str) -> bool {
    url.starts_with("https://")
}

pub fn list_subscriptions(path: &Path, _config: &AppConfig) {
    if let Ok(subs) = load_subs(path) {
        if subs.is_empty() {
            println!("Нет подписок");
            return;
        }
        println!("═══════════════════════ Подписки ═══════════════════════");
        println!("  {:<4} {:<20} {:>2} {:>6} {:>12}  {:<30} {:>12}", 
                 "ID", "Имя", "TS", "Живых", "Обновлено", "Ссылка", "Автообн.");
        for sub in subs {
            let live_path = dirs::config_dir().unwrap().join("vpn-manager").join(format!("sub_{}_live.txt", sub.id));
            let count = if live_path.exists() {
                fs::read_to_string(&live_path).map(|s| s.lines().count()).unwrap_or(0)
            } else { 0 };
            let ts_path = dirs::config_dir().unwrap().join("vpn-manager").join(format!("sub_{}_timestamp.txt", sub.id));
            let updated = fs::read_to_string(&ts_path).unwrap_or_else(|_| "—".into());
            let short_url = shorten_url(&sub.url);
            let insecure_mark = if is_https(&sub.url) { "🔒" } else { "🔓" };
            let auto_update = if _config.auto_update_interval > 0 && 
                (_config.auto_update_ids == "all" || utils::expand_ids(&_config.auto_update_ids).unwrap_or_default().contains(&sub.id)) 
            { format!("{}м", _config.auto_update_interval) } else { "—".to_string() };

            let color = if !sub.enabled || count == 0 {
                "\x1b[31m" // тёмно-красный для отключённых/пустых
            } else {
                ""
            };
            println!("  {}{:<4} {:<20.20} {:<2} {:>6} {:>12}  {:<30.30} {:>12}\x1b[0m",
                     color, sub.id, sub.name, insecure_mark, count, updated.trim(), short_url, auto_update);
        }
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

    // Попытка автоматически получить имя
    let name = if let Ok(resp) = reqwest::blocking::get(&url) {
        if let Ok(body) = resp.text() {
            // Ищем первую строку вида # Название или что-то похожее
            body.lines()
                .find(|l| l.starts_with('#') && !l.starts_with("##"))
                .map(|l| l.trim_start_matches('#').trim().to_string())
                .or_else(|| {
                    // Если нет, извлекаем из URL: последний сегмент пути
                    url::Url::parse(&url).ok()
                        .and_then(|u| u.path_segments()?.last().map(|s| s.to_string()))
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
    subs.push(Subscription { id: new_id, name, url, enabled: true });
    subs.sort_by_key(|s| s.id);
    save_subs(path, &subs)?;
    println!("✅ Подписка добавлена");
    Ok(())
}

pub fn edit_subscription(path: &Path, id: usize) -> Result<()> {
    let mut subs = load_subs(path)?;
    let sub = subs.iter_mut().find(|s| s.id == id).context("Подписка не найдена")?;
    println!("Текущие данные: ID={}, Имя={}, URL={}", sub.id, sub.name, sub.url);
    print!("Новое имя (Enter = оставить): ");
    io::stdout().flush()?;
    let mut name = String::new();
    io::stdin().read_line(&mut name)?;
    print!("Новый URL (Enter = оставить): ");
    io::stdout().flush()?;
    let mut url = String::new();
    io::stdin().read_line(&mut url)?;
    if !name.trim().is_empty() { sub.name = name.trim().into(); }
    if !url.trim().is_empty() { sub.url = url.trim().into(); }
    save_subs(path, &subs)?;
    println!("✅ Подписка изменена");
    Ok(())
}

pub fn remove_subscriptions(path: &Path, ids_spec: &str) -> Result<()> {
    let mut subs = load_subs(path)?;
    let ids = if ids_spec == "all" { (1..=subs.len()).collect() } else { utils::expand_ids(ids_spec)? };
    let mut removed = 0;
    subs.retain(|s| if ids.contains(&s.id) { removed += 1; false } else { true });
    for (i, s) in subs.iter_mut().enumerate() { s.id = i + 1; }
    save_subs(path, &subs)?;
    println!("✅ Удалено подписок: {}", removed);
    Ok(())
}

pub fn switch_subscriptions(path: &Path, ids_spec: &str, enable: bool) -> Result<()> {
    let mut subs = load_subs(path)?;
    let ids = if ids_spec == "all" { (1..=subs.len()).collect() } else { utils::expand_ids(ids_spec)? };
    for sub in subs.iter_mut() { if ids.contains(&sub.id) { sub.enabled = enable; } }
    save_subs(path, &subs)?;
    let action = if enable { "включены" } else { "выключены" };
    println!("✅ Подписки {}: {}", action, ids_spec);
    Ok(())
}

pub fn update_subscriptions(target: &str, protocol: &str, limit: usize, keep_raw: bool, show_info: bool, config: &AppConfig, subs_path: &Path) -> Result<()> {
    let subs = load_subs(subs_path)?;
    let ids = if target == "all" { subs.iter().map(|s| s.id).collect() } else { utils::expand_ids(target)? };
    for id in ids {
        if let Some(sub) = subs.iter().find(|s| s.id == id) {
            update::update_single_sub(sub, protocol, limit, keep_raw, show_info, config)?;
        } else {
            eprintln!("⚠️ Подписка {} не найдена", id);
        }
    }
    Ok(())
}

pub fn handle_subs_ext(action: Option<SubsCmd>, subs_path: &Path, config: &AppConfig) -> Result<()> {
    match action.unwrap_or(SubsCmd::List) {
        SubsCmd::List => {
            list_subscriptions(subs_path, config);
            print_subs_help();
            Ok(())
        }
        SubsCmd::Add => add_subscription(subs_path),
        SubsCmd::Edit { id } => edit_subscription(subs_path, id),
        SubsCmd::Remove { ids } => remove_subscriptions(subs_path, &ids),
        SubsCmd::Update { target, info, protocol, limit, keep_raw } => {
            match target {
                Some(t) => update_subscriptions(&t, &protocol, limit, keep_raw, info, config, subs_path)?,
                None => {
                    list_subscriptions(subs_path, config);
                    print_subs_help();
                }
            }
            Ok(())
        }
        SubsCmd::Switch { action } => match action {
            SwitchCmd::On { ids } => switch_subscriptions(subs_path, &ids, true),
            SwitchCmd::Off { ids } => switch_subscriptions(subs_path, &ids, false),
        },
        SubsCmd::Cfscanner { sub_id, args } => {
            let mut full_args = vec![];
            if let Some(sid) = sub_id {
                let config_dir = dirs::config_dir().unwrap().join("vpn-manager");
                let proxy_config = config_dir.join(format!("sub_{}_live.txt", sid));
                if proxy_config.exists() {
                    full_args.push("-C".to_string());
                    full_args.push(proxy_config.to_string_lossy().into());
                } else {
                    anyhow::bail!("Нет живых конфигов для подписки {}. Сначала выполните subs update {}", sid, sid);
                }
            }
            full_args.extend(args);
            crate::run_xray_knife("cfscanner", &full_args)
        }
    }
}

fn print_subs_help() {
    println!("Команды subs:");
    println!("  list                    показать список (по умолчанию)");
    println!("  add                     добавить подписку");
    println!("  edit <ID>               изменить подписку");
    println!("  remove <IDs>            удалить (all, 1,2-4, 1,3,5)");
    println!("  update <IDs> [-i] [-p proto] [-l N] [-k on|off]");
    println!("  switch on/off <IDs>     включить/выключить");
    println!("  cfscanner [--sub-id ID] [флаги]");
}
