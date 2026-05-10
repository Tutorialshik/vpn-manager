use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{self, BufRead, Write};
use std::path::Path;

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
            serde_json::from_value(serde_json::Value::Array(subs.clone()))
                .context("Ошибка десериализации подписок")
        } else if v.is_array() {
            serde_json::from_value(v).context("Ошибка десериализации подписок")
        } else {
            bail!("Неверный формат subscriptions.json")
        }
    } else {
        Ok(vec![]) // пустой список
    }
}

fn save_subs(path: &Path, subs: &Subscriptions) -> Result<()> {
    let obj = serde_json::json!({ "subscriptions": subs });
    let json = serde_json::to_string_pretty(&obj)?;
    fs::write(path, json)?;
    Ok(())
}

pub fn list_subscriptions(path: &Path, _config: &super::config::AppConfig) {
    if let Ok(subs) = load_subs(path) {
        if subs.is_empty() {
            println!("Нет подписок");
            return;
        }
        println!("═══════════════════════ Подписки ═══════════════════════");
        println!("  {:<4} {:<25} {:>6} {:>16}", "ID", "Имя", "Живых", "Обновлено");
        for sub in subs {
            if sub.enabled {
                let live_path = dirs::config_dir()
                    .unwrap()
                    .join("vpn-manager")
                    .join(format!("sub_{}_live.txt", sub.id));
                let count = if live_path.exists() {
                    fs::read_to_string(&live_path)
                        .map(|s| s.lines().count())
                        .unwrap_or(0)
                } else { 0 };
                // timestamp упрощённо
                let ts_path = dirs::config_dir()
                    .unwrap()
                    .join("vpn-manager")
                    .join(format!("sub_{}_timestamp.txt", sub.id));
                let updated = fs::read_to_string(&ts_path).unwrap_or_else(|_| "—".into());
                println!("  {:<4} {:<25.25} {:>6} {:>16}", sub.id, sub.name, count, updated.trim());
            }
        }
    }
}

pub fn handle_subs(action: Option<super::SubsCmd>, path: &Path, _config: &super::config::AppConfig) -> Result<()> {
    match action {
        None => {
        list_subscriptions(path, _config),
        Some(super::SubsCmd::Add) => {
            let mut id_str = String::new();
            print!("ID (пусто = авто): ");
            io::stdout().flush()?;
            io::stdin().read_line(&mut id_str)?;
            let id: usize = if id_str.trim().is_empty() {
                // авто ID
                let subs = load_subs(path)?;
                subs.iter().map(|s| s.id).max().unwrap_or(0) + 1
            } else {
                id_str.trim().parse()?
            };
            print!("Имя: ");
            io::stdout().flush()?;
            let mut name = String::new();
            io::stdin().read_line(&mut name)?;
            print!("URL: ");
            io::stdout().flush()?;
            let mut url = String::new();
            io::stdin().read_line(&mut url)?;

            let mut subs = load_subs(path)?;
            if let Some(sub) = subs.iter_mut().find(|s| s.id == id) {
                sub.name = name.trim().into();
                sub.url = url.trim().into();
            } else {
                subs.push(Subscription {
                    id,
                    name: name.trim().into(),
                    url: url.trim().into(),
                    enabled: true,
                });
            }
            subs.sort_by_key(|s| s.id);
            save_subs(path, &subs)?;
            println!("✅ Подписка добавлена/обновлена");
            Ok(())
        }
    }
        Some(super::SubsCmd::Remove { id }) => {
            let mut subs = load_subs(path)?;
            if let Some(pos) = subs.iter().position(|s| s.id == id) {
                subs.remove(pos);
                // перенумерация
                for (i, s) in subs.iter_mut().enumerate() {
                    s.id = i + 1;
                }
                save_subs(path, &subs)?;
                println!("✅ Подписка {} удалена", id);
            } else {
                eprintln!("❌ Подписка {} не найдена", id);
            }
            Ok(())
        }
        Some(super::SubsCmd::Edit { id }) => {
            let mut subs = load_subs(path)?;
            let sub = subs.iter_mut().find(|s| s.id == id)
                .context("Подписка не найдена")?;
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
        Some(super::SubsCmd::List) => {
            list_subscriptions(path, _config);
            Ok(())
        }
    }
}
