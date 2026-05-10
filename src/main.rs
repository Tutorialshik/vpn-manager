mod config;
mod commands;
mod geo;
mod proxy;
mod settings;
mod subs;
mod update;
mod utils;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use config::AppConfig;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "vpn-manager", version, about = "Управление VPN подписками и прокси")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    #[command(about = "Запуск прокси")]
    Start {
        #[arg(required = true)]
        target: String,
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        extra_args: Vec<String>,
    },
    #[command(about = "Остановка прокси")]
    Stop,
    #[command(about = "Перезапуск последнего профиля")]
    Restart {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        extra_args: Vec<String>,
    },
    #[command(about = "Обновление подписок")]
    Update {
        target: Option<String>,
        #[arg(short = 'p', long = "protocol", default_value_t = String::from("all"))]
        protocol: String,
        #[arg(short = 'l', long = "limit", default_value_t = 0)]
        limit: usize,
        #[arg(short = 'k', long = "keep-raw")]
        keep_raw: bool,
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

#[derive(Subcommand)]
enum SubsCmd {
    Add,
    Remove { id: usize },
    Edit { id: usize },
    List,
}

#[derive(Subcommand)]
enum SettingsCmd {
    Port { value: u16 },
    Proto { value: String },
    Mode { value: String },
    Core { value: String },
    Rotate { seconds: u64 },
    Insecure { on: bool },
    Speedtest { on: bool },
    HttpVerbose { on: bool },
    #[command(subcommand)]
    HttpUrls(HttpUrlsCmd),
    BlacklistDuration { seconds: u64 },
    BlacklistStrikes { strikes: u32 },
    AutoUpdate { interval_min: u64, ids: Option<String> },
    SelectMode { mode: String },
    MenuUpdate { interval_sec: u64 },
    MenuPosition { pos: usize },
    Reset,
}

#[derive(Subcommand)]
enum HttpUrlsCmd {
    Show,
    Add { url: String },
    Remove { id: usize },
    Activate { ids: String },
    Deactivate { ids: String },
}

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
    let cmd_help = commands::load_commands(&commands_path).ok(); // не критично

    match cli.command {
        Some(Commands::Start { target, extra_args }) => {
            proxy::handle_start(&target, &extra_args, &mut app_config, &config_path, &subs_path)?;
        }
        Some(Commands::Stop) => {
            proxy::stop_proxy(&app_config)?;
        }
        Some(Commands::Restart { extra_args }) => {
            proxy::stop_proxy(&app_config)?;
            // используем last_region из app_config
            proxy::handle_start("now", &extra_args, &mut app_config, &config_path, &subs_path)?;
        }
        Some(Commands::Update { target, protocol, limit, keep_raw }) => {
            let target = target.unwrap_or_else(|| "all".into());
            update::handle_update(&target, &protocol, limit, keep_raw, &app_config, &subs_path)?;
        }
        Some(Commands::Subs { action }) => {
            subs::handle_subs(action, &subs_path, &app_config)?;
        }
        Some(Commands::Settings { setting }) => {
            settings::handle_settings(setting, &mut app_config, &config_path, &subs_path)?;
        }
        Some(Commands::Status) => {
            status(&app_config, &subs_path)?;
        }
        None => {
            if let Some(help) = cmd_help {
                println!("{}", help.global_usage());
            } else {
                println!("Не загружены описания команд. Используйте --help");
            }
        }
    }

    Ok(())
}

fn status(app: &AppConfig, subs_path: &PathBuf) -> Result<()> {
    use proxy::is_running;
    println!("═══════════════════ Состояние ═══════════════════");
    println!("  Порт:            {}", app.default_port);
    println!("  Регион:          {}", app.last_region);
    println!("  Режим:           {}", app.last_mode_type);
    println!("  Ядро:            {}", app.core);
    // ... остальные поля аналогично
    if is_running(app) {
        println!("  Прокси:          запущен");
    } else {
        println!("  Прокси:          не запущен");
    }
    subs::list_subscriptions(subs_path, app);
    Ok(())
}
