use anyhow::{Result, bail};
use std::fs;
use std::path::Path;
use crate::config::AppConfig;

// В main мы уже определили SettingsCmd. Здесь обрабатываем.

pub fn handle_settings(setting: Option<super::SettingsCmd>, config: &mut AppConfig, config_path: &Path, subs_path: &Path) -> Result<()> {
    match setting {
        Some(super::SettingsCmd::Port { value }) => {
            config.default_port = value;
            save_and_log(config, config_path, &format!("Порт: {}", value))?;
        }
        Some(super::SettingsCmd::Proto { value }) => {
            config.last_inbound_proto = value;
            save_and_log(config, config_path, &format!("Протокол: {}", config.last_inbound_proto))?;
        }
        Some(super::SettingsCmd::Mode { value }) => {
            config.last_mode_type = value;
            save_and_log(config, config_path, &format!("Режим: {}", config.last_mode_type))?;
        }
        Some(super::SettingsCmd::Core { value }) => {
            config.core = value;
            save_and_log(config, config_path, &format!("Ядро: {}", config.core))?;
        }
        Some(super::SettingsCmd::Rotate { seconds }) => {
            config.rotate = seconds;
            save_and_log(config, config_path, &format!("Ротация: {}{}", seconds, "с"))?;
        }
        Some(super::SettingsCmd::Insecure { on }) => {
            config.insecure = on;
            save_and_log(config, config_path, &format!("Insecure: {}", if on { "вкл" } else { "выкл" }))?;
        }
        Some(super::SettingsCmd::Speedtest { on }) => {
            config.speedtest = on;
            save_and_log(config, config_path, &format!("Speedtest: {}", if on { "вкл" } else { "выкл" }))?;
        }
        Some(super::SettingsCmd::HttpVerbose { on }) => {
            config.http_verbose = on;
            save_and_log(config, config_path, &format!("HTTP verbose: {}", if on { "вкл" } else { "выкл" }))?;
        }
        Some(super::SettingsCmd::HttpUrls(cmd)) => {
            handle_http_urls(cmd, config, config_path)?;
        }
        Some(super::SettingsCmd::BlacklistDuration { seconds }) => {
            config.blacklist_duration = seconds;
            save_and_log(config, config_path, &format!("Длительность блэклиста: {}{}", seconds, "с"))?;
        }
        Some(super::SettingsCmd::BlacklistStrikes { strikes }) => {
            config.blacklist_strikes = strikes;
            save_and_log(config, config_path, &format!("Блэклист ошибок: {}", strikes))?;
        }
        Some(super::SettingsCmd::AutoUpdate { interval_min, ids }) => {
            config.auto_update_interval = interval_min;
            config.auto_update_ids = ids.unwrap_or_else(|| "all".into());
            // cron setup не реализован для краткости, но можно добавить
            save_and_log(config, config_path, "Автообновление обновлено")?;
        }
        Some(super::SettingsCmd::SelectMode { mode }) => {
            if mode != "random" && mode != "fastest" {
                bail!("mode должен быть random или fastest");
            }
            config.select_mode = mode;
            save_and_log(config, config_path, &format!("Режим выбора: {}", config.select_mode))?;
        }
        Some(super::SettingsCmd::MenuUpdate { interval_sec }) => {
            config.menu_update_interval = interval_sec;
            save_and_log(config, config_path, "Автообновление меню обновлено")?;
        }
        Some(super::SettingsCmd::MenuPosition { pos }) => {
            config.menu_position = pos;
            save_and_log(config, config_path, &format!("Позиция меню: {}", pos))?;
        }
        Some(super::SettingsCmd::Reset) => {
            *config = AppConfig::default();
            config.save(config_path)?;
            println!("✅ Сброшено на умолчания");
        }
        None => {
            // показать текущие настройки
            println!("Текущие настройки:");
            println!("  Порт:            {}", config.default_port);
            println!("  Протокол:        {}", config.last_inbound_proto);
            println!("  Режим:           {}", config.last_mode_type);
            println!("  Ядро:            {}", config.core);
            // ... остальные поля
        }
    }
    Ok(())
}

fn save_and_log(config: &mut AppConfig, path: &Path, msg: &str) -> Result<()> {
    config.save(path)?;
    println!("✅ {}", msg);
    Ok(())
}

fn handle_http_urls(cmd: super::HttpUrlsCmd, config: &mut AppConfig, config_path: &Path) -> Result<()> {
    match cmd {
        super::HttpUrlsCmd::Show => {
            println!("Пул тестовых URL:");
            // парсить HTTP_URL_POOL_DATA и отобразить с отметкой активных
        }
        super::HttpUrlsCmd::Add { url } => {
            // добавить url
            // сохранить
        }
        super::HttpUrlsCmd::Remove { id } => {
            // удалить
        }
        super::HttpUrlsCmd::Activate { ids } => {
            config.http_url_active_ids = ids;
            config.save(config_path)?;
        }
        super::HttpUrlsCmd::Deactivate { ids } => {
            // логика деактивации
        }
    }
    Ok(())
}
