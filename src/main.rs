mod commands;
mod config;
mod db;
mod geo;
mod http_tester;
mod knife;
mod parallel_rules;
mod proxy;
mod settings;
mod subs;
mod update;
mod utils;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use config::AppConfig;
use http_tester::XrayKnifeHttpTester;
use std::path::{Path, PathBuf};

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
        #[command(subcommand)]
        target: Option<StartTarget>,

        #[arg(short = 'm', long = "method")]
        method: Option<String>,

        #[arg(short = 'r', long = "rotate")]
        rotate: Option<u64>,

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

// ── start ──────────────────────────────────────────────

#[derive(Subcommand, Clone)]
pub enum StartTarget {
    #[command(about = "Показать меню выбора профиля")]
    Menu,
    #[command(about = "Запустить последний профиль")]
    Now,
    #[command(about = "Запустить региональный файл (ru, eu, de, ...)")]
    Region { region: String },
    #[command(about = "Запустить живые конфиги подписки по ID")]
    Sub { id: usize },
    #[command(about = "Выполнить команду в неймспейсе (xray-knife exec)")]
    Exec { args: Vec<String> },
}

// ── change ─────────────────────────────────────────────

#[derive(Subcommand, Clone)]
pub enum ChangeCmd {
    /// Следующий профиль в текущем списке
    Next,
    /// Предыдущий профиль
    Prev,
    /// Случайный профиль
    Random,
    /// Самый быстрый профиль (по данным статистики)
    Fastest,
}

// ── subs ───────────────────────────────────────────────

#[derive(Subcommand, Clone)]
pub enum SubsCmd {
    #[command(about = "Показать список подписок и справку")]
    List,
    #[command(about = "Добавить подписку (интерактивно)")]
    Add,
    #[command(about = "Изменить подписку по ID")]
    Edit { id: usize },
    #[command(about = "Удалить подписки (all, 1, 2-4, 1,3,5)")]
    Remove { ids: String },
    #[command(about = "Обновить подписки (all, 1, 2-4, 1,3,5)")]
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
        /// Загружать подписку через запущенный прокси (SOCKS5 на 127.0.0.1:порт)
        #[arg(short = 'x', long = "via-proxy", default_value_t = false)]
        via_proxy: bool,
    },
    #[command(about = "Включить/выключить подписки")]
    Switch {
        #[command(subcommand)]
        action: SwitchCmd,
    },
    #[command(about = "Сканер Cloudflare IP")]
    Cfscanner {
        #[arg(long = "sub-id")]
        sub_id: Option<usize>,
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
}

#[derive(Subcommand, Clone)]
pub enum SwitchCmd {
    #[command(about = "Включить подписки (1,2-4, all)")]
    On { ids: String },
    #[command(about = "Выключить подписки (1,2-4, all)")]
    Off { ids: String },
}

// ── settings ───────────────────────────────────────────

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

// ── main ───────────────────────────────────────────────

fn main() -> Result<()> {
    env_logger::init();
    let cli = Cli::parse();

    let config_dir = dirs::config_dir()
        .context("Не удалось определить config директорию")?
        .join("vpn-manager");
    std::fs::create_dir_all(&config_dir).context("Не удалось создать директорию конфига")?;

    let config_path = config_dir.join("config.json");
    let commands_path = config_dir.join("commands.json");
    let subs_path = config_dir.join("subscriptions.json");

    let mut app_config = AppConfig::load_or_default(&config_path)?;
    let cmd_help = commands::load_commands(&commands_path).ok();

    let db_path = PathBuf::from(&app_config.parallel_db_path);
    let db = db::open_db(&db_path).ok();

    match cli.command {
        Some(Commands::Start {
            target,
            method,
            rotate,
            extra_args,
        }) => {
            let target = target.unwrap_or(StartTarget::Menu);
            let mut proxy_args = extra_args.clone();
            if let Some(m) = method {
                proxy_args.push("-m".into());
                proxy_args.push(m);
            }
            if let Some(r) = rotate {
                proxy_args.push("-r".into());
                proxy_args.push(r.to_string());
            }
            match target {
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
                    knife::run_knife("exec", &args)?;
                }
            }
        }
        Some(Commands::Stop) => {
            proxy::stop_proxy(&app_config)?;
        }
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
                println!("Не загружены описания команд. Используйте --help");
            }
        }
    }

    Ok(())
}

fn status(app: &AppConfig, subs_path: &Path, _db: Option<&rusqlite::Connection>) -> Result<()> {
    let running = proxy::is_running(app);
    let (status_text, color) = if running {
        ("запущен", "\x1b[32m")
    } else {
        ("не запущен", "\x1b[31m")
    };
    println!("═══════════════════ Состояние ═══════════════════");
    println!("  Порт:            {}", app.default_port);
    println!("  Регион:          {}", app.last_region);
    println!("  Режим:           {}", app.last_mode_type);
    println!("  Ядро:            {}", app.core);
    println!("  Ротация:         {} с", app.rotate);
    println!(
        "  Insecure:        {}",
        if app.insecure { "вкл" } else { "выкл" }
    );
    println!(
        "  Speedtest:       {}",
        if app.speedtest { "вкл" } else { "выкл" }
    );
    println!(
        "  HTTP verbose:    {}",
        if app.http_verbose {
            "вкл"
        } else {
            "выкл"
        }
    );
    println!("  Прокси:          {}{}\x1b[0m", color, status_text);

    if let Some(info) = proxy::get_current_config_info(app) {
        println!(
            "  Текущий сервер:  {} {} ({})",
            info.flag, info.country, info.host
        );
        println!(
            "  IP:              {} (протокол: {})",
            info.ip, info.protocol
        );
    }

    subs::list_subscriptions(subs_path, app);
    Ok(())
}
