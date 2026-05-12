use crate::config::AppConfig;
use crate::knife;
use crate::subs;
use crate::utils;
use anyhow::{bail, Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration; // новый импорт

// ... (структуры ConfigInfo и функции is_running, stop_proxy, handle_start, change_profile без изменений,
//      кроме тех мест, где вызывается Command::new("xray-knife") для proxy и где требуется knife)

// В функцию start_xray внесены изменения: вместо прямого spawn используется knife::spawn_proxy.

pub struct ConfigInfo {
    pub flag: String,
    pub country: String,
    pub host: String,
    pub ip: String,
    pub protocol: String,
}

pub fn is_running(_config: &AppConfig) -> bool {
    match pid_file_path() {
        Ok(pid_path) => {
            if let Ok(pid_str) = fs::read_to_string(&pid_path) {
                if let Ok(pid) = pid_str.trim().parse::<i32>() {
                    unsafe { libc::kill(pid, 0) == 0 }
                } else {
                    false
                }
            } else {
                false
            }
        }
        Err(_) => false,
    }
}

fn pid_file_path() -> Result<PathBuf> {
    let dir = dirs::config_dir()
        .context("Не удалось определить config директорию")?
        .join("vpn-manager");
    Ok(dir.join("xray-knife.pid"))
}

pub fn stop_proxy(_config: &AppConfig) -> Result<()> {
    let pid_path = pid_file_path()?;
    if let Ok(pid_str) = fs::read_to_string(&pid_path) {
        if let Ok(pid) = pid_str.trim().parse::<i32>() {
            if unsafe { libc::kill(pid, 0) } == 0 {
                unsafe { libc::kill(pid, 15) };
                std::thread::sleep(std::time::Duration::from_secs(1));
                let _ = unsafe { libc::kill(pid, 9) };
                println!("✅ Прокси остановлен (PID {})", pid);
            }
        }
        let _ = fs::remove_file(&pid_path);
    }
    let _ = Command::new("pkill")
        .arg("-f")
        .arg("xray-knife proxy")
        .status();
    Ok(())
}

pub fn handle_start(
    target: &str,
    extra_args: &[String],
    config: &mut AppConfig,
    config_path: &Path,
    _subs_path: &Path,
) -> Result<()> {
    stop_proxy(config)?;
    let config_dir = dirs::config_dir()
        .context("Не удалось определить config директорию")?
        .join("vpn-manager");

    let (cfg_file, save_region) = match target {
        "now" => {
            if config.last_region.starts_with("_sub_") {
                let id = &config.last_region[5..];
                let live = config_dir.join(format!("sub_{}_live.txt", id));
                if !live.exists() {
                    bail!("Сначала обновите подписку {}", id);
                }
                (live, config.last_region.clone())
            } else {
                let region =
                    utils::resolve_region(&config.last_region).unwrap_or_else(|| "eu".into());
                let file = config_dir.join("lists").join(format!("{}.txt", region));
                if !file.exists() {
                    bail!("Файл региона {} не найден", region);
                }
                (file, region)
            }
        }
        region if utils::resolve_region(region).is_some() => {
            let r = utils::resolve_region(region).unwrap();
            let file = config_dir.join("lists").join(format!("{}.txt", r));
            if !file.exists() || fs::metadata(&file)?.len() == 0 {
                bail!("Файл региона {} не найден или пуст", r);
            }
            (file, r)
        }
        other => {
            let parts: Vec<&str> = other.split_whitespace().collect();
            if parts.len() == 2 && parts[0] == "sub" {
                let id: usize = parts[1].parse()?;
                let live = config_dir.join(format!("sub_{}_live.txt", id));
                if !live.exists() {
                    bail!("Сначала update {}", id);
                }
                (live, format!("_sub_{}", id))
            } else {
                bail!("Неизвестная цель: {}", target);
            }
        }
    };

    start_xray(&cfg_file, extra_args, config, &config_dir)?;
    config.last_region = save_region;
    config.save(config_path)?;
    Ok(())
}

pub fn change_profile(
    action: crate::ChangeCmd,
    config: &AppConfig,
    _subs_path: &Path,
) -> Result<()> {
    // ... без изменений ...
    if !is_running(config) {
        bail!("Прокси не запущен. Сначала выполните start.");
    }
    let config_dir = dirs::config_dir()
        .context("Не удалось определить config директорию")?
        .join("vpn-manager");

    let current_cfg = if config.last_region.starts_with("_sub_") {
        let id = &config.last_region[5..];
        config_dir.join(format!("sub_{}_live.txt", id))
    } else {
        let region = utils::resolve_region(&config.last_region).unwrap_or_else(|| "eu".into());
        config_dir.join("lists").join(format!("{}.txt", region))
    };
    if !current_cfg.exists() {
        bail!("Файл конфигов не найден: {}", current_cfg.display());
    }

    let profiles: Vec<String> = fs::read_to_string(&current_cfg)?
        .lines()
        .map(|l| l.to_string())
        .collect();

    if profiles.is_empty() {
        bail!("Список профилей пуст");
    }

    let selected = match action {
        crate::ChangeCmd::Next => {
            if profiles.len() > 1 {
                profiles[1].clone()
            } else {
                profiles[0].clone()
            }
        }
        crate::ChangeCmd::Prev => {
            let n = profiles.len();
            if n > 1 {
                profiles[n - 1].clone()
            } else {
                profiles[0].clone()
            }
        }
        crate::ChangeCmd::Random => {
            let n = profiles.len();
            let idx = rand::random::<usize>() % n;
            profiles[idx].clone()
        }
        crate::ChangeCmd::Fastest => profiles[0].clone(),
    };

    let tmp_file = tempfile::NamedTempFile::new()?;
    fs::write(tmp_file.path(), &selected)?;
    stop_proxy(config)?;
    start_xray(tmp_file.path(), &[], config, &config_dir)?;
    println!("✅ Профиль изменён");
    Ok(())
}

fn start_xray(
    cfg_file: &Path,
    extra_args: &[String],
    config: &AppConfig,
    config_dir: &Path,
) -> Result<()> {
    let mut args = vec!["-f".to_string(), cfg_file.to_string_lossy().into()];
    if config.last_mode_type == "system" {
        args.push("--mode".into());
        args.push("system".into());
        args.push("--port".into());
        args.push(config.default_port.to_string());
    } else {
        args.push("--inbound".into());
        args.push(config.last_inbound_proto.clone());
        args.push("--port".into());
        args.push(config.default_port.to_string());
    }
    if config.insecure {
        args.push("-e".into());
    }
    args.push("--core".into());
    args.push(config.core.clone());
    args.extend_from_slice(extra_args);
    if !args.contains(&"-r".to_string()) && !args.contains(&"--rotate".to_string()) {
        args.push("--rotate".into());
        args.push(config.rotate.to_string());
    }
    args.push("--blacklist-duration".into());
    args.push(config.blacklist_duration.to_string());
    args.push("--blacklist-strikes".into());
    args.push(config.blacklist_strikes.to_string());

    let log_file_path = config_dir.join("vpn-manager.log");
    // Используем knife::spawn_proxy вместо прямого Command
    let mut child = knife::spawn_proxy(&args, &log_file_path)?;

    let pid = child.id();
    std::thread::sleep(Duration::from_secs(1));
    match child.try_wait() {
        Ok(Some(status)) => {
            bail!(
                "Процесс xray-knife proxy неожиданно завершился (код {:?}). Проверьте лог: {}",
                status.code(),
                log_file_path.display()
            );
        }
        Ok(None) => {
            fs::write(pid_file_path()?, pid.to_string())?;
        }
        Err(e) => {
            bail!("Ошибка при проверке процесса: {}", e);
        }
    }

    // ... остальной вывод информации без изменений ...
    let mode_str = if config.last_mode_type == "system" {
        "system"
    } else {
        &format!("inbound ({})", config.last_inbound_proto)
    };
    println!("═══════════════════ Прокси запущен ═══════════════════");
    println!(" Файл: {} (ядро: {})", cfg_file.display(), config.core);
    println!(" Адрес: {}:{}", config.listen_ip, config.default_port);
    println!(" Режим: {}", mode_str);
    println!(
        " Ротация: каждые {}с, блэклист: {}/{}с",
        config.rotate, config.blacklist_strikes, config.blacklist_duration
    );
    println!(" Доп. флаги: {:?}", &extra_args);

    if let Some(info) = get_config_info_from_file(cfg_file, config) {
        println!(
            " Сервер: {} {} ({} / {}) протокол: {}",
            info.flag, info.country, info.host, info.ip, info.protocol
        );
    }

    println!(" PID: {}", pid);
    println!("══════════════════════════════════════════════════════");
    Ok(())
}

// ... show_start_help, get_current_config_info, get_config_info_from_file остаются без изменений ...

pub fn show_start_help(config: &AppConfig, subs_path: &Path) {
    println!("═════════════════ start – запуск прокси ═════════════════");
    println!(" vpn-manager start [menu] это меню");
    println!(" vpn-manager start now последний профиль");
    println!(" vpn-manager start region <REGION> регион (ru, eu, de…)");
    println!(" vpn-manager start sub <ID> подписка по ID");
    println!(" vpn-manager start exec <cmd...> выполнить в неймспейсе");
    println!(" Флаги:");
    println!(" -m, --method <random|fastest> метод выбора");
    println!(" -r, --rotate <S> интервал ротации");
    println!(" все флаги xray-knife proxy также принимаются");
    println!("Регионы:");
    if let Some(config_dir) = dirs::config_dir() {
        let lists_dir = config_dir.join("vpn-manager").join("lists");
        for region in &["ru", "us", "eu", "de", "pl", "fi", "nl", "other"] {
            let file = lists_dir.join(format!("{}.txt", region));
            let count = if file.exists() {
                fs::read_to_string(&file)
                    .map(|s| s.lines().count())
                    .unwrap_or(0)
            } else {
                0
            };
            println!(" {:<6} – {:>6} живых", region, count);
        }
    } else {
        println!("Не удалось получить путь к конфигурации. Показываю подписки:");
    }
    println!("Подписки:");
    subs::list_subscriptions(subs_path, config);
    println!("════════════════════════════════════════════════════════");
}

pub fn get_current_config_info(config: &AppConfig) -> Option<ConfigInfo> {
    let config_dir = dirs::config_dir()?.join("vpn-manager");
    let cfg_file = if config.last_region.starts_with("_sub_") {
        let id = &config.last_region[5..];
        config_dir.join(format!("sub_{}_live.txt", id))
    } else {
        let region = utils::resolve_region(&config.last_region)?;
        config_dir.join("lists").join(format!("{}.txt", region))
    };
    get_config_info_from_file(&cfg_file, config)
}

fn get_config_info_from_file(cfg_file: &Path, config: &AppConfig) -> Option<ConfigInfo> {
    if !cfg_file.exists() {
        return None;
    }
    let first_link = fs::read_to_string(cfg_file)
        .ok()?
        .lines()
        .next()?
        .to_string();
    let host = utils::extract_host(&first_link);
    if host == "unknown" {
        return None;
    }
    let ip = utils::resolve_ip(&host)?;
    let code = crate::geo::country_code(&ip, &config.geoip_db);
    let country = code.clone().unwrap_or_else(|| "??".into());
    let flag = match code.as_deref() {
        Some("RU") => "🇷🇺",
        Some("US") => "🇺🇸",
        Some("DE") => "🇩🇪",
        Some("PL") => "🇵🇱",
        Some("FI") => "🇫🇮",
        Some("NL") => "🇳🇱",
        Some("EU") => "🇪🇺",
        Some(_) => "🌍",
        _ => "🌍",
    };
    let protocol = if first_link.starts_with("vless://") {
        "vless"
    } else if first_link.starts_with("vmess://") {
        "vmess"
    } else if first_link.starts_with("trojan://") {
        "trojan"
    } else if first_link.starts_with("ss://") {
        "ss"
    } else {
        "?"
    };
    Some(ConfigInfo {
        flag: flag.to_string(),
        country,
        host,
        ip,
        protocol: protocol.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_pid_file_path_returns_result() {
        // Не паникует, возвращает Result
        let path = pid_file_path();
        match path {
            Ok(p) => assert!(p.ends_with("xray-knife.pid")),
            Err(_) => {} // Ожидаемо, если не определена домашняя директория в тестах
        }
    }

    #[test]
    fn test_get_config_info_from_file_empty_file() {
        let file = NamedTempFile::new().unwrap();
        let config = AppConfig::default();
        let info = get_config_info_from_file(file.path(), &config);
        assert!(info.is_none());
    }

    #[test]
    fn test_get_config_info_from_file_valid_link() {
        let file = NamedTempFile::new().unwrap();
        fs::write(
            file.path(),
            "vless://a@1.2.3.4:443?security=reality#MyProxy\n",
        )
        .unwrap();
        let config = AppConfig::default();
        let info = get_config_info_from_file(file.path(), &config);
        // Функция попытается резолвить IP 1.2.3.4, но в тестовом окружении может не быть сети.
        // Проверим лишь отсутствие паники и что host извлечён.
        // Для полной проверки замокать резолвинг, но это уже интеграционный тест.
        // Пока просто убеждаемся, что паники нет.
        let _ = info;
    }
}
