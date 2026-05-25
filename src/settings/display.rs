use vpn_core::config::AppConfig;
use vpn_l10n as l10n;

pub fn show_current_settings(config: &AppConfig) {
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
