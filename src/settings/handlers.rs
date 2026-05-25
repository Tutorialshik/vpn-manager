use anyhow::Result;
use std::path::Path;
use vpn_core::config::AppConfig;
use vpn_l10n as l10n;

use super::{display, http_urls, validation};

pub fn handle_settings(
    setting: Option<crate::SettingsCmd>,
    config: &mut AppConfig,
    config_path: &Path,
    _subs_path: &Path,
) -> Result<()> {
    match setting {
        Some(crate::SettingsCmd::Port { value }) => {
            set_port(value, config, config_path)?;
        }
        Some(crate::SettingsCmd::Ip { value }) => {
            set_ip(value, config, config_path)?;
        }
        Some(crate::SettingsCmd::Proto { value }) => {
            set_proto(value, config, config_path)?;
        }
        Some(crate::SettingsCmd::Method { value }) => {
            validation::validate_method(&value)?;
            set_method(value, config, config_path)?;
        }
        Some(crate::SettingsCmd::Mode { value }) => {
            set_mode(value, config, config_path)?;
        }
        Some(crate::SettingsCmd::Core { value }) => {
            set_core(value, config, config_path)?;
        }
        Some(crate::SettingsCmd::Rotate { seconds }) => {
            set_rotate(seconds, config, config_path)?;
        }
        Some(crate::SettingsCmd::Insecure { on }) => {
            set_insecure(on, config, config_path)?;
        }
        Some(crate::SettingsCmd::Speedtest { on }) => {
            set_speedtest(on, config, config_path)?;
        }
        Some(crate::SettingsCmd::HttpVerbose { on }) => {
            set_http_verbose(on, config, config_path)?;
        }
        Some(crate::SettingsCmd::Info { on }) => {
            set_info(on, config, config_path)?;
        }
        Some(crate::SettingsCmd::Parallel { value }) => {
            set_parallel(value, config, config_path)?;
        }
        Some(crate::SettingsCmd::HttpUrls(cmd)) => {
            http_urls::handle_http_urls(cmd, config, config_path)?;
        }
        Some(crate::SettingsCmd::BlacklistDuration { seconds }) => {
            set_blacklist_duration(seconds, config, config_path)?;
        }
        Some(crate::SettingsCmd::BlacklistStrikes { strikes }) => {
            set_blacklist_strikes(strikes, config, config_path)?;
        }
        Some(crate::SettingsCmd::AutoUpdate { interval_min, ids }) => {
            set_auto_update(interval_min, ids, config, config_path)?;
        }
        Some(crate::SettingsCmd::AutoMenuUpdate {
            enable,
            interval_sec,
        }) => {
            set_auto_menu_update(enable, interval_sec, config, config_path)?;
        }
        Some(crate::SettingsCmd::Reset) => {
            reset_config(config, config_path)?;
        }
        None => {
            display::show_current_settings(config);
        }
    }
    Ok(())
}

// --- отдельные сеттеры ---
fn save_and_log(config: &mut AppConfig, path: &Path, msg: &str) -> Result<()> {
    config.save(path)?;
    println!("{}", l10n::t_fmt("settings.saved", &[msg]));
    Ok(())
}

fn set_port(value: u16, config: &mut AppConfig, path: &Path) -> Result<()> {
    config.default_port = value;
    save_and_log(
        config,
        path,
        &l10n::t_fmt("settings.port_saved", &[&value.to_string()]),
    )
}

fn set_ip(value: String, config: &mut AppConfig, path: &Path) -> Result<()> {
    config.listen_ip = value;
    save_and_log(
        config,
        path,
        &l10n::t_fmt("settings.ip_saved", &[&config.listen_ip]),
    )
}

fn set_proto(value: String, config: &mut AppConfig, path: &Path) -> Result<()> {
    config.last_inbound_proto = value;
    save_and_log(
        config,
        path,
        &l10n::t_fmt("settings.proto_saved", &[&config.last_inbound_proto]),
    )
}

fn set_method(value: String, config: &mut AppConfig, path: &Path) -> Result<()> {
    config.select_mode = value;
    save_and_log(
        config,
        path,
        &l10n::t_fmt("settings.method_saved", &[&config.select_mode]),
    )
}

fn set_mode(value: String, config: &mut AppConfig, path: &Path) -> Result<()> {
    config.last_mode_type = value;
    save_and_log(
        config,
        path,
        &l10n::t_fmt("settings.mode_saved", &[&config.last_mode_type]),
    )
}

fn set_core(value: String, config: &mut AppConfig, path: &Path) -> Result<()> {
    config.core = value;
    save_and_log(
        config,
        path,
        &l10n::t_fmt("settings.core_saved", &[&config.core]),
    )
}

fn set_rotate(seconds: u64, config: &mut AppConfig, path: &Path) -> Result<()> {
    config.rotate = seconds;
    save_and_log(
        config,
        path,
        &l10n::t_fmt("settings.rotate_saved", &[&seconds.to_string()]),
    )
}

fn set_insecure(on: bool, config: &mut AppConfig, path: &Path) -> Result<()> {
    config.insecure = on;
    let state = if on {
        l10n::t("common.yes")
    } else {
        l10n::t("common.no")
    };
    save_and_log(
        config,
        path,
        &l10n::t_fmt("settings.insecure_saved", &[&state]),
    )
}

fn set_speedtest(on: bool, config: &mut AppConfig, path: &Path) -> Result<()> {
    config.speedtest = on;
    let state = if on {
        l10n::t("common.yes")
    } else {
        l10n::t("common.no")
    };
    save_and_log(
        config,
        path,
        &l10n::t_fmt("settings.speedtest_saved", &[&state]),
    )
}

fn set_http_verbose(on: bool, config: &mut AppConfig, path: &Path) -> Result<()> {
    config.http_verbose = on;
    let state = if on {
        l10n::t("common.yes")
    } else {
        l10n::t("common.no")
    };
    save_and_log(
        config,
        path,
        &l10n::t_fmt("settings.http_verbose_saved", &[&state]),
    )
}

fn set_info(on: bool, config: &mut AppConfig, path: &Path) -> Result<()> {
    config.show_update_info = on;
    let state = if on {
        l10n::t("common.yes")
    } else {
        l10n::t("common.no")
    };
    save_and_log(
        config,
        path,
        &l10n::t_fmt("settings.show_info_saved", &[&state]),
    )
}

fn set_parallel(value: usize, config: &mut AppConfig, path: &Path) -> Result<()> {
    config.parallel_tests = value;
    save_and_log(
        config,
        path,
        &l10n::t_fmt("settings.parallel_saved", &[&value.to_string()]),
    )
}

fn set_blacklist_duration(seconds: u64, config: &mut AppConfig, path: &Path) -> Result<()> {
    config.blacklist_duration = seconds;
    save_and_log(
        config,
        path,
        &l10n::t_fmt("settings.blacklist_duration_saved", &[&seconds.to_string()]),
    )
}

fn set_blacklist_strikes(strikes: u32, config: &mut AppConfig, path: &Path) -> Result<()> {
    config.blacklist_strikes = strikes;
    save_and_log(
        config,
        path,
        &l10n::t_fmt("settings.blacklist_strikes_saved", &[&strikes.to_string()]),
    )
}

fn set_auto_update(
    interval_min: u64,
    ids: Option<String>,
    config: &mut AppConfig,
    path: &Path,
) -> Result<()> {
    config.auto_update_interval = interval_min;
    config.auto_update_ids = ids.unwrap_or_else(|| "all".into());
    save_and_log(
        config,
        path,
        &l10n::t_fmt(
            "settings.auto_update_saved",
            &[
                &config.auto_update_interval.to_string(),
                &config.auto_update_ids,
            ],
        ),
    )
}

fn set_auto_menu_update(
    enable: bool,
    interval_sec: u64,
    config: &mut AppConfig,
    path: &Path,
) -> Result<()> {
    config.auto_menu_update_enabled = enable;
    config.auto_menu_update_interval = interval_sec;
    let status = if enable {
        format!("включено, интервал {} с", interval_sec)
    } else {
        "выключено".into()
    };
    save_and_log(
        config,
        path,
        &l10n::t_fmt("settings.auto_menu_update_saved", &[&status]),
    )
}

fn reset_config(config: &mut AppConfig, path: &Path) -> Result<()> {
    *config = AppConfig::default();
    config.save(path)?;
    println!("{}", l10n::t("settings.reset"));
    Ok(())
}
