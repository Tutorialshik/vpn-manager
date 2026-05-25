use crate::l10n;
use anyhow::{bail, Result};
use std::path::Path;
use vpn_core::config::AppConfig;

pub fn handle_settings(
    setting: Option<super::SettingsCmd>,
    config: &mut AppConfig,
    config_path: &Path,
    _subs_path: &Path,
) -> Result<()> {
    match setting {
        Some(super::SettingsCmd::Port { value }) => {
            config.default_port = value;
            save_and_log(
                config,
                config_path,
                &l10n::t_fmt("settings.port_saved", &[&value.to_string()]),
            )?;
        }
        Some(super::SettingsCmd::Ip { value }) => {
            config.listen_ip = value;
            save_and_log(
                config,
                config_path,
                &l10n::t_fmt("settings.ip_saved", &[&config.listen_ip]),
            )?;
        }
        Some(super::SettingsCmd::Proto { value }) => {
            config.last_inbound_proto = value;
            save_and_log(
                config,
                config_path,
                &l10n::t_fmt("settings.proto_saved", &[&config.last_inbound_proto]),
            )?;
        }
        Some(super::SettingsCmd::Method { value }) => {
            if value != "random" && value != "fastest" {
                bail!(l10n::t("settings.invalid_method"));
            }
            config.select_mode = value;
            save_and_log(
                config,
                config_path,
                &l10n::t_fmt("settings.method_saved", &[&config.select_mode]),
            )?;
        }
        Some(super::SettingsCmd::Mode { value }) => {
            config.last_mode_type = value;
            save_and_log(
                config,
                config_path,
                &l10n::t_fmt("settings.mode_saved", &[&config.last_mode_type]),
            )?;
        }
        Some(super::SettingsCmd::Core { value }) => {
            config.core = value;
            save_and_log(
                config,
                config_path,
                &l10n::t_fmt("settings.core_saved", &[&config.core]),
            )?;
        }
        Some(super::SettingsCmd::Rotate { seconds }) => {
            config.rotate = seconds;
            save_and_log(
                config,
                config_path,
                &l10n::t_fmt("settings.rotate_saved", &[&seconds.to_string()]),
            )?;
        }
        Some(super::SettingsCmd::Insecure { on }) => {
            config.insecure = on;
            let state = if on {
                l10n::t("common.yes")
            } else {
                l10n::t("common.no")
            };
            save_and_log(
                config,
                config_path,
                &l10n::t_fmt("settings.insecure_saved", &[&state]),
            )?;
        }
        Some(super::SettingsCmd::Speedtest { on }) => {
            config.speedtest = on;
            let state = if on {
                l10n::t("common.yes")
            } else {
                l10n::t("common.no")
            };
            save_and_log(
                config,
                config_path,
                &l10n::t_fmt("settings.speedtest_saved", &[&state]),
            )?;
        }
        Some(super::SettingsCmd::HttpVerbose { on }) => {
            config.http_verbose = on;
            let state = if on {
                l10n::t("common.yes")
            } else {
                l10n::t("common.no")
            };
            save_and_log(
                config,
                config_path,
                &l10n::t_fmt("settings.http_verbose_saved", &[&state]),
            )?;
        }
        Some(super::SettingsCmd::Info { on }) => {
            config.show_update_info = on;
            let state = if on {
                l10n::t("common.yes")
            } else {
                l10n::t("common.no")
            };
            save_and_log(
                config,
                config_path,
                &l10n::t_fmt("settings.show_info_saved", &[&state]),
            )?;
        }
        Some(super::SettingsCmd::Parallel { value }) => {
            config.parallel_tests = value;
            save_and_log(
                config,
                config_path,
                &l10n::t_fmt("settings.parallel_saved", &[&value.to_string()]),
            )?;
        }
        Some(super::SettingsCmd::HttpUrls(cmd)) => {
            handle_http_urls(cmd, config, config_path)?;
        }
        Some(super::SettingsCmd::BlacklistDuration { seconds }) => {
            config.blacklist_duration = seconds;
            save_and_log(
                config,
                config_path,
                &l10n::t_fmt("settings.blacklist_duration_saved", &[&seconds.to_string()]),
            )?;
        }
        Some(super::SettingsCmd::BlacklistStrikes { strikes }) => {
            config.blacklist_strikes = strikes;
            save_and_log(
                config,
                config_path,
                &l10n::t_fmt("settings.blacklist_strikes_saved", &[&strikes.to_string()]),
            )?;
        }
        Some(super::SettingsCmd::AutoUpdate { interval_min, ids }) => {
            config.auto_update_interval = interval_min;
            config.auto_update_ids = ids.unwrap_or_else(|| "all".into());
            save_and_log(
                config,
                config_path,
                &l10n::t_fmt(
                    "settings.auto_update_saved",
                    &[
                        &config.auto_update_interval.to_string(),
                        &config.auto_update_ids,
                    ],
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
                &l10n::t_fmt("settings.auto_menu_update_saved", &[&status]),
            )?;
        }
        Some(super::SettingsCmd::Reset) => {
            *config = AppConfig::default();
            config.save(config_path)?;
            println!("{}", l10n::t("settings.reset"));
        }
        None => {
            show_current_settings(config);
        }
    }
    Ok(())
}

fn save_and_log(config: &mut AppConfig, path: &Path, msg: &str) -> Result<()> {
    config.save(path)?;
    println!("{}", l10n::t_fmt("settings.saved", &[msg]));
    Ok(())
}

fn show_current_settings(config: &AppConfig) {
    println!("{}", l10n::t("settings.current_title"));
    println!(
        "{}",
        l10n::t_fmt("settings.port", &[&config.default_port.to_string()])
    );
    println!("{}", l10n::t_fmt("settings.ip", &[&config.listen_ip]));
    println!(
        "{}",
        l10n::t_fmt("settings.proto", &[&config.last_inbound_proto])
    );
    println!("{}", l10n::t_fmt("settings.method", &[&config.select_mode]));
    println!(
        "{}",
        l10n::t_fmt("settings.mode", &[&config.last_mode_type])
    );
    println!("{}", l10n::t_fmt("settings.core", &[&config.core]));
    println!(
        "{}",
        l10n::t_fmt("settings.rotate", &[&config.rotate.to_string()])
    );
    println!(
        "{}",
        l10n::t_fmt(
            "settings.insecure",
            &[if config.insecure {
                l10n::t("common.yes")
            } else {
                l10n::t("common.no")
            }
            .as_str()]
        )
    );
    println!(
        "{}",
        l10n::t_fmt(
            "settings.speedtest",
            &[if config.speedtest {
                l10n::t("common.yes")
            } else {
                l10n::t("common.no")
            }
            .as_str()]
        )
    );
    println!(
        "{}",
        l10n::t_fmt(
            "settings.http_verbose",
            &[if config.http_verbose {
                l10n::t("common.yes")
            } else {
                l10n::t("common.no")
            }
            .as_str()]
        )
    );
    println!(
        "{}",
        l10n::t_fmt(
            "settings.show_info",
            &[if config.show_update_info {
                l10n::t("common.yes")
            } else {
                l10n::t("common.no")
            }
            .as_str()]
        )
    );
    println!(
        "{}",
        l10n::t_fmt(
            "settings.parallel_tests",
            &[&config.parallel_tests.to_string()]
        )
    );
    println!(
        "{}",
        l10n::t_fmt(
            "settings.blacklist",
            &[
                &config.blacklist_strikes.to_string(),
                &config.blacklist_duration.to_string()
            ]
        )
    );
    println!(
        "{}",
        l10n::t_fmt(
            "settings.auto_update",
            &[
                &config.auto_update_interval.to_string(),
                &config.auto_update_ids
            ]
        )
    );
    let auto_menu = if config.auto_menu_update_enabled {
        format!("{} с", config.auto_menu_update_interval)
    } else {
        l10n::t("common.no")
    };
    println!(
        "{}",
        l10n::t_fmt("settings.auto_menu_update", &[&auto_menu])
    );
    println!(
        "{}",
        l10n::t_fmt("settings.active_urls", &[&config.http_url_active_ids])
    );
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
            println!(
                "{}",
                l10n::t_fmt("settings.saved", &["Активные URL обновлены"])
            );
        }
        super::HttpUrlsCmd::Deactivate { ids: _ } => {}
    }
    Ok(())
}
