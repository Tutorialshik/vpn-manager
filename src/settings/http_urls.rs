use anyhow::Result;
use std::path::Path;
use vpn_core::config::AppConfig;

pub fn handle_http_urls(
    cmd: crate::HttpUrlsCmd,
    config: &mut AppConfig,
    config_path: &Path,
) -> Result<()> {
    match cmd {
        crate::HttpUrlsCmd::Show => {
            println!("Активные тестовые URL: {}", config.http_url_active_ids);
            println!("Доступные URL (ID|URL):");
            for entry in config.http_url_pool_data.split(';') {
                if let Some((id, url)) = entry.split_once('|') {
                    let active = if config.http_url_active_ids.contains(id) {
                        "✅"
                    } else {
                        "⭕"
                    };
                    println!("  {} {} -> {}", active, id, url);
                }
            }
        }
        crate::HttpUrlsCmd::Add { url } => {
            let next_id = config
                .http_url_pool_data
                .split(';')
                .filter(|s| !s.is_empty())
                .count()
                + 1;
            let new_entry = format!("{next_id}|{url}");
            if !config.http_url_pool_data.is_empty() {
                config.http_url_pool_data.push(';');
            }
            config.http_url_pool_data.push_str(&new_entry);
            config.save(config_path)?;
            println!("✅ URL добавлен с ID {}", next_id);
        }
        crate::HttpUrlsCmd::Remove { id } => {
            let mut entries: Vec<&str> = config.http_url_pool_data.split(';').collect();
            let before = entries.len();
            entries.retain(|e| !e.starts_with(&format!("{}|", id)));
            if entries.len() == before {
                println!("⚠️ ID {} не найден", id);
            } else {
                config.http_url_pool_data = entries.join(";");
                let active_ids: Vec<&str> = config.http_url_active_ids.split(',').collect();
                let new_active = active_ids
                    .into_iter()
                    .filter(|&i| i != id.to_string())
                    .collect::<Vec<_>>()
                    .join(",");
                config.http_url_active_ids = new_active;
                config.save(config_path)?;
                println!("✅ URL с ID {} удалён", id);
            }
        }
        crate::HttpUrlsCmd::Activate { ids } => {
            config.http_url_active_ids = ids;
            config.save(config_path)?;
            println!("✅ Активные URL обновлены: {}", config.http_url_active_ids);
        }
        crate::HttpUrlsCmd::Deactivate { ids } => {
            let to_deactivate: Vec<&str> = ids.split(',').collect();
            let active_ids: Vec<&str> = config.http_url_active_ids.split(',').collect();
            let new_active: Vec<String> = active_ids
                .into_iter()
                .filter(|id| !to_deactivate.contains(id))
                .map(String::from)
                .collect();
            config.http_url_active_ids = new_active.join(",");
            config.save(config_path)?;
            println!("✅ ID деактивированы: {}", ids);
        }
    }
    Ok(())
}
