mod commands;
mod db;
mod geo;
mod l10n;
mod proxy;
mod settings;
mod subs;
mod update;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};
use vpn_core::config::AppConfig;
use vpn_testing::XrayKnifeHttpTester;

#[derive(Parser)]
#[command(
    name = "vpn-manager",
    version,
    about = "Управление VPN подписками и прокси"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    #[command(about = "Запуск прокси")]
    Start {
        #[arg(short = 'm', long = "method")]
        method: Option<String>,

        #[arg(short = 'r', long = "rotate")]
        rotate: Option<u64>,

        /// Цель и дополнительные аргументы (menu, now, region <регион>, sub <ID>, exec <команда...>)
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        extra_args: Vec<String>,
    },
    #[command(about = "Остановка прокси")]
    Stop,
    #[command(about = "Перезапуск последнего профиля")]
    Restart,
    #[command(about = "Сменить профиль не прерывая сессию")]
    Change {
        #[command(subcommand)]
        action: Option<ChangeCmd>,
    },
    #[command(about = "Управление подписками")]
    Subs {
        #[command(subcommand)]
        action: Option<SubsCmd>,
    },
    #[command(about = "Изменение настроек")]
    Settings {
        #[command(subcommand)]
        setting: Option<SettingsCmd>,
    },
    #[command(about = "Показать состояние")]
    Status,
}

#[derive(Subcommand, Clone)]
pub enum ChangeCmd {
    Next,
    Prev,
    Random,
    Fastest,
}

#[derive(Subcommand, Clone)]
pub enum SubsCmd {
    List,
    Add,
    Edit {
        id: usize,
    },
    Remove {
        ids: String,
    },
    Update {
        #[arg(required = false)]
        target: Option<String>,
        #[arg(short = 'i', long = "info")]
        info: bool,
        #[arg(short = 'p', long = "protocol", default_value_t = String::from("all"))]
        protocol: String,
        #[arg(short = 'l', long = "limit", default_value_t = 0)]
        limit: usize,
        #[arg(short = 'k', long = "keep-raw", default_value_t = true)]
        keep_raw: bool,
        #[arg(short = 'x', long = "via-proxy", default_value_t = false)]
        via_proxy: bool,
    },
    Switch {
        #[command(subcommand)]
        action: SwitchCmd,
    },
    Cfscanner {
        #[arg(long = "sub-id")]
        sub_id: Option<usize>,
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
}

#[derive(Subcommand, Clone)]
pub enum SwitchCmd {
    On { ids: String },
    Off { ids: String },
}

#[derive(Subcommand)]
pub enum SettingsCmd {
    Port {
        value: u16,
    },
    Ip {
        value: String,
    },
    Proto {
        value: String,
    },
    Method {
        value: String,
    },
    Mode {
        value: String,
    },
    Core {
        value: String,
    },
    Rotate {
        seconds: u64,
    },
    Insecure {
        on: bool,
    },
    Speedtest {
        on: bool,
    },
    HttpVerbose {
        on: bool,
    },
    Info {
        on: bool,
    },
    Parallel {
        value: usize,
    },
    #[command(subcommand)]
    HttpUrls(HttpUrlsCmd),
    BlacklistDuration {
        seconds: u64,
    },
    BlacklistStrikes {
        strikes: u32,
    },
    AutoUpdate {
        interval_min: u64,
        ids: Option<String>,
    },
    AutoMenuUpdate {
        #[arg(short = 'e', long = "enable")]
        enable: bool,
        interval_sec: u64,
    },
    Reset,
}

#[derive(Subcommand)]
pub enum HttpUrlsCmd {
    Show,
    Add { url: String },
    Remove { id: usize },
    Activate { ids: String },
    Deactivate { ids: String },
}

fn main() -> Result<()> {
    env_logger::init();

    // Инициализация локализации
    let config_dir = dirs::config_dir()
        .context(l10n::t("main.dir_missing"))?
        .join("vpn-manager");
    std::fs::create_dir_all(&config_dir).context(l10n::t("main.create_dir_fail"))?;
    let locales_dir = config_dir.join("locales");
    std::fs::create_dir_all(&locales_dir)?;
    let locale_path = locales_dir.join("ru.json");
    if !locale_path.exists() {
        if let Some(embedded) = option_env!("EMBED_LOCALE_RU") {
            std::fs::write(&locale_path, embedded)?;
        } else {
            let src = PathBuf::from("locales/ru.json");
            if src.exists() {
                std::fs::copy(&src, &locale_path)?;
            }
        }
    }
    l10n::init(&locale_path)?;

    let cli = Cli::parse();

    let config_path = config_dir.join("config.json");
    let commands_path = config_dir.join("commands.json");
    let subs_path = config_dir.join("subscriptions.json");

    let mut app_config = AppConfig::load_or_default(&config_path)?;
    let cmd_help = commands::load_commands(&commands_path).ok();

    let db_path = PathBuf::from(&app_config.parallel_db_path);
    let db = db::open_db(&db_path).ok();

    match cli.command {
        Some(Commands::Start {
            method,
            rotate,
            extra_args,
        }) => {
            let target = if extra_args.is_empty() {
                "menu".to_string()
            } else {
                extra_args[0].clone()
            };
            let mut proxy_args: Vec<String> = if extra_args.len() > 1 {
                extra_args[1..].to_vec()
            } else {
                vec![]
            };

            if let Some(m) = method {
                proxy_args.push("-m".into());
                proxy_args.push(m);
            }
            if let Some(r) = rotate {
                proxy_args.push("-r".into());
                proxy_args.push(r.to_string());
            }

            let target_parsed = parse_start_target(&target, &mut proxy_args)?;
            match target_parsed {
                StartTarget::Menu => proxy::show_start_help(&app_config, &subs_path),
                StartTarget::Now => {
                    proxy::handle_start(
                        "now",
                        &proxy_args,
                        &mut app_config,
                        &config_path,
                        &subs_path,
                    )?;
                }
                StartTarget::Region { region } => {
                    proxy::handle_start(
                        &region,
                        &proxy_args,
                        &mut app_config,
                        &config_path,
                        &subs_path,
                    )?;
                }
                StartTarget::Sub { id } => {
                    proxy::handle_start(
                        &format!("sub {}", id),
                        &proxy_args,
                        &mut app_config,
                        &config_path,
                        &subs_path,
                    )?;
                }
                StartTarget::Exec { args } => {
                    vpn_knife::run_knife("exec", &args)?;
                }
            }
        }
        Some(Commands::Stop) => proxy::stop_proxy(&app_config)?,
        Some(Commands::Restart) => {
            proxy::stop_proxy(&app_config)?;
            proxy::handle_start("now", &[], &mut app_config, &config_path, &subs_path)?;
        }
        Some(Commands::Change { action }) => {
            let action = action.unwrap_or(ChangeCmd::Next);
            proxy::change_profile(action, &app_config, &subs_path)?;
        }
        Some(Commands::Subs { action }) => {
            let tester = XrayKnifeHttpTester;
            subs::handle_subs_ext(action, &subs_path, &app_config, db.as_ref(), &tester)?;
        }
        Some(Commands::Settings { setting }) => {
            settings::handle_settings(setting, &mut app_config, &config_path, &subs_path)?;
        }
        Some(Commands::Status) => {
            status(&app_config, &subs_path, db.as_ref())?;
        }
        None => {
            if let Some(help) = cmd_help {
                println!("{}", help.global_help.global_usage());
            } else {
                println!("{}", l10n::t("main.no_help"));
            }
        }
    }

    Ok(())
}

enum StartTarget {
    Menu,
    Now,
    Region { region: String },
    Sub { id: usize },
    Exec { args: Vec<String> },
}

fn parse_start_target(target: &str, proxy_args: &mut Vec<String>) -> Result<StartTarget> {
    let trimmed = target.trim();
    if trimmed.is_empty() || trimmed == "menu" {
        Ok(StartTarget::Menu)
    } else if trimmed == "now" {
        Ok(StartTarget::Now)
    } else if trimmed == "region" {
        if proxy_args.is_empty() {
            Err(anyhow::anyhow!(l10n::t_fmt(
                "proxy.unknown_target",
                &[target]
            )))
        } else {
            let region = proxy_args.remove(0);
            Ok(StartTarget::Region { region })
        }
    } else if trimmed == "sub" {
        if proxy_args.is_empty() {
            Err(anyhow::anyhow!(l10n::t_fmt(
                "proxy.unknown_target",
                &[target]
            )))
        } else {
            let id: usize = proxy_args.remove(0).parse()?;
            Ok(StartTarget::Sub { id })
        }
    } else if trimmed == "exec" {
        let args = std::mem::take(proxy_args);
        Ok(StartTarget::Exec { args })
    } else if vpn_core::utils::resolve_region(trimmed).is_some() {
        Ok(StartTarget::Region {
            region: trimmed.to_owned(),
        })
    } else {
        Err(anyhow::anyhow!(l10n::t_fmt(
            "proxy.unknown_target",
            &[target]
        )))
    }
}

fn status(app: &AppConfig, subs_path: &Path, _db: Option<&rusqlite::Connection>) -> Result<()> {
    let running = proxy::is_running(app);
    let (status_text, color) = if running {
        (l10n::t("status.running"), "\x1b[32m")
    } else {
        (l10n::t("status.not_running"), "\x1b[31m")
    };
    println!("{}", l10n::t("status.title"));
    println!(
        "{}",
        l10n::t_fmt("status.port", &[&app.default_port.to_string()])
    );
    println!("{}", l10n::t_fmt("status.region", &[&app.last_region]));
    println!("{}", l10n::t_fmt("status.mode", &[&app.last_mode_type]));
    println!("{}", l10n::t_fmt("status.core", &[&app.core]));
    println!(
        "{}",
        l10n::t_fmt("status.rotate", &[&app.rotate.to_string()])
    );
    println!(
        "{}",
        l10n::t_fmt(
            "status.insecure",
            &[if app.insecure {
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
            "status.speedtest",
            &[if app.speedtest {
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
            "status.http_verbose",
            &[if app.http_verbose {
                l10n::t("common.yes")
            } else {
                l10n::t("common.no")
            }
            .as_str()]
        )
    );
    println!(
        "{}",
        l10n::t_fmt("status.proxy_running", &[color, &status_text])
    );

    if let Some(info) = proxy::get_current_config_info(app) {
        println!(
            "{}",
            l10n::t_fmt("status.server", &[&info.flag, &info.country, &info.host])
        );
        println!(
            "{}",
            l10n::t_fmt("status.ip_proto", &[&info.ip, &info.protocol])
        );
    }

    subs::list_subscriptions(subs_path, app);
    Ok(())
}
