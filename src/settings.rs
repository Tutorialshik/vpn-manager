use crate::config::AppConfig;
use anyhow::{bail, Result};
use std::path::Path;

pub fn handle_settings(
    setting: Option<super::SettingsCmd>,
    config: &mut AppConfig,
    config_path: &Path,
    _subs_path: &Path,
) -> Result<()> {
    match setting {
        Some(super::SettingsCmd::Port { value }) => {
            config.default_port = value;
            save_and_log(config, config_path, &format!("Порт: {}", value))?;
        }
        Some(super::SettingsCmd::Ip { value }) => {
            config.listen_ip = value;
            save_and_log(
                config,
                config_path,
                &format!("IP прослушивания: {}", config.listen_ip),
            )?;
        }
        Some(super::SettingsCmd::Proto { value }) => {
            config.last_inbound_proto = value;
            save_and_log(
                config,
                config_path,
                &format!("Протокол: {}", config.last_inbound_proto),
            )?;
        }
        Some(super::SettingsCmd::Method { value }) => {
            if value != "random" && value != "fastest" {
                bail!("Метод должен быть random или fastest");
            }
            config.select_mode = value;
            save_and_log(
                config,
                config_path,
                &format!("Метод выбора: {}", config.select_mode),
            )?;
        }
        Some(super::SettingsCmd::Mode { value }) => {
            config.last_mode_type = value;
            save_and_log(
                config,
                config_path,
                &format!("Режим: {}", config.last_mode_type),
            )?;
        }
        Some(super::SettingsCmd::Core { value }) => {
            config.core = value;
            save_and_log(config, config_path, &format!("Ядро: {}", config.core))?;
        }
        Some(super::SettingsCmd::Rotate { seconds }) => {
            config.rotate = seconds;
            save_and_log(config, config_path, &format!("Ротация: {}с", seconds))?;
        }
        Some(super::SettingsCmd::Insecure { on }) => {
            config.insecure = on;
            save_and_log(
                config,
                config_path,
                &format!("Insecure: {}", if on { "вкл" } else { "выкл" }),
            )?;
        }
        Some(super::SettingsCmd::Speedtest { on }) => {
            config.speedtest = on;
            save_and_log(
                config,
                config_path,
                &format!("Speedtest: {}", if on { "вкл" } else { "выкл" }),
            )?;
        }
        Some(super::SettingsCmd::HttpVerbose { on }) => {
            config.http_verbose = on;
            save_and_log(
                config,
                config_path,
                &format!("HTTP verbose: {}", if on { "вкл" } else { "выкл" }),
            )?;
        }
        Some(super::SettingsCmd::Info { on }) => {
            config.show_update_info = on;
            save_and_log(
                config,
                config_path,
                &format!(
                    "Показывать инфо при обновлении: {}",
                    if on { "вкл" } else { "выкл" }
                ),
            )?;
        }
        Some(super::SettingsCmd::Parallel { value }) => {
            config.parallel_tests = value;
            save_and_log(
                config,
                config_path,
                &format!("Параллельных тестов: {}", value),
            )?;
        }
        Some(super::SettingsCmd::HttpUrls(cmd)) => {
            handle_http_urls(cmd, config, config_path)?;
        }
        Some(super::SettingsCmd::BlacklistDuration { seconds }) => {
            config.blacklist_duration = seconds;
            save_and_log(config, config_path, &format!("Блэклист: {} с", seconds))?;
        }
        Some(super::SettingsCmd::BlacklistStrikes { strikes }) => {
            config.blacklist_strikes = strikes;
            save_and_log(
                config,
                config_path,
                &format!("Ошибок до блэклиста: {}", strikes),
            )?;
        }
        Some(super::SettingsCmd::AutoUpdate { interval_min, ids }) => {
            config.auto_update_interval = interval_min;
            config.auto_update_ids = ids.unwrap_or_else(|| "all".into());
            save_and_log(
                config,
                config_path,
                &format!(
                    "Автообновление: {} мин для {}",
                    config.auto_update_interval, config.auto_update_ids
                ),
            )?;
        }
        Some(super::SettingsCmd::AutoMenuUpdate {
            enable,
            interval_sec,
        }) => {
            config.auto_menu_update_enabled = enable;
            config.auto_menu_update_interval = interval_sec;
            let status = if enable {
                format!("включено, интервал {} с", interval_sec)
            } else {
                "выключено".into()
            };
            save_and_log(
                config,
                config_path,
                &format!("Автообновление меню: {}", status),
            )?;
        }
        Some(super::SettingsCmd::Reset) => {
            *config = AppConfig::default();
            config.save(config_path)?;
            println!("✅ Сброшено на умолчания");
        }
        None => {
            show_current_settings(config);
        }
    }
    Ok(())
}

fn save_and_log(config: &mut AppConfig, path: &Path, msg: &str) -> Result<()> {
    config.save(path)?;
    println!("✅ {}", msg);
    Ok(())
}

fn show_current_settings(config: &AppConfig) {
    println!("Текущие настройки:");
    println!("  Порт:            {}", config.default_port);
    println!("  IP:              {}", config.listen_ip);
    println!("  Протокол:        {}", config.last_inbound_proto);
    println!("  Метод:           {}", config.select_mode);
    println!("  Режим:           {}", config.last_mode_type);
    println!("  Ядро:            {}", config.core);
    println!("  Ротация:         {} с", config.rotate);
    println!(
        "  Insecure:        {}",
        if config.insecure {
            "вкл"
        } else {
            "выкл"
        }
    );
    println!(
        "  Speedtest:       {}",
        if config.speedtest {
            "вкл"
        } else {
            "выкл"
        }
    );
    println!(
        "  HTTP verbose:    {}",
        if config.http_verbose {
            "вкл"
        } else {
            "выкл"
        }
    );
    println!(
        "  Показ инфо при обн.: {}",
        if config.show_update_info {
            "вкл"
        } else {
            "выкл"
        }
    );
    println!("  Параллельные тесты: {}", config.parallel_tests);
    println!(
        "  Блэклист:        ошибок {}, длит. {} с",
        config.blacklist_strikes, config.blacklist_duration
    );
    println!(
        "  Автообновление:  {} мин, IDs: {}",
        config.auto_update_interval, config.auto_update_ids
    );
    println!(
        "  Автообновление меню: {}",
        if config.auto_menu_update_enabled {
            format!("{} с", config.auto_menu_update_interval)
        } else {
            "выкл".into()
        }
    );
    println!("  Активные URL:    {}", config.http_url_active_ids);
}

fn handle_http_urls(
    cmd: super::HttpUrlsCmd,
    config: &mut AppConfig,
    config_path: &Path,
) -> Result<()> {
    match cmd {
        super::HttpUrlsCmd::Show => {
            println!("Пул тестовых URL (в разработке)");
        }
        super::HttpUrlsCmd::Add { url: _ } => {}
        super::HttpUrlsCmd::Remove { id: _ } => {}
        super::HttpUrlsCmd::Activate { ids } => {
            config.http_url_active_ids = ids;
            config.save(config_path)?;
            println!("✅ Активные URL обновлены");
        }
        super::HttpUrlsCmd::Deactivate { ids: _ } => {}
    }
    Ok(())
}
