use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use crate::config::AppConfig;
use crate::subs;
use crate::utils;

pub struct ConfigInfo {
    pub flag: String,
    pub country: String,
    pub host: String,
    pub ip: String,
    pub protocol: String,
}

pub fn is_running(_config: &AppConfig) -> bool {
    let pid_path = pid_file_path();
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

fn pid_file_path() -> PathBuf {
    dirs::config_dir()
        .unwrap()
        .join("vpn-manager")
        .join("xray-knife.pid")
}

pub fn stop_proxy(_config: &AppConfig) -> Result<()> {
    let pid_path = pid_file_path();
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
    let _ = Command::new("pkill").arg("-f").arg("xray-knife proxy").status();
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

    let (cfg_file, save_region) = match target {
        "now" => {
            if config.last_region.starts_with("_sub_") {
                let id = &config.last_region[5..];
                let live = dirs::config_dir()
                    .unwrap()
                    .join("vpn-manager")
                    .join(format!("sub_{}_live.txt", id));
                if !live.exists() {
                    anyhow::bail!("Сначала обновите подписку {}", id);
                }
                (live, config.last_region.clone())
            } else {
                let region = utils::resolve_region(&config.last_region)
                    .unwrap_or_else(|| "eu".into());
                let file = dirs::config_dir()
                    .unwrap()
                    .join("vpn-manager")
                    .join("lists")
                    .join(format!("{}.txt", region));
                if !file.exists() {
                    anyhow::bail!("Файл региона {} не найден", region);
                }
                (file, region)
            }
        }
        region if utils::resolve_region(region).is_some() => {
            let r = utils::resolve_region(region).unwrap();
            let file = dirs::config_dir()
                .unwrap()
                .join("vpn-manager")
                .join("lists")
                .join(format!("{}.txt", r));
            if !file.exists() || fs::metadata(&file)?.len() == 0 {
                anyhow::bail!("Файл региона {} не найден или пуст", r);
            }
            (file, r)
        }
        other => {
            let parts: Vec<&str> = other.split_whitespace().collect();
            if parts.len() == 2 && parts[0] == "sub" {
                let id: usize = parts[1].parse()?;
                let live = dirs::config_dir()
                    .unwrap()
                    .join("vpn-manager")
                    .join(format!("sub_{}_live.txt", id));
                if !live.exists() {
                    anyhow::bail!("Сначала update {}", id);
                }
                (live, format!("_sub_{}", id))
            } else {
                anyhow::bail!("Неизвестная цель: {}", target);
            }
        }
    };

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
    if config.insecure { args.push("-e".into()); }
    args.push("--core".into()); args.push(config.core.clone());
    // пользовательские аргументы (rotate, method и т.п.)
    args.extend_from_slice(extra_args);
    // настройки менеджера имеют меньший приоритет, если пользователь не указал свои
    if !args.contains(&"-r".to_string()) && !args.contains(&"--rotate".to_string()) {
        args.push("--rotate".into());
        args.push(config.rotate.to_string());
    }
    args.push("--blacklist-duration".into());
    args.push(config.blacklist_duration.to_string());
    args.push("--blacklist-strikes".into());
    args.push(config.blacklist_strikes.to_string());

    let log_file_path = dirs::config_dir().unwrap().join("vpn-manager").join("vpn-manager.log");
    let log_file = fs::File::create(&log_file_path)?;
    let child = Command::new("xray-knife")
        .arg("proxy")
        .args(&args)
        .stdout(Stdio::from(log_file.try_clone()?))
        .stderr(Stdio::from(log_file))
        .spawn()
        .context("Не удалось запустить xray-knife")?;

    let pid = child.id();
    fs::write(pid_file_path(), pid.to_string())?;

    config.last_region = save_region;
    config.save(config_path)?;

    println!("✅ Прокси запущен (PID {})", pid);
    Ok(())
}

pub fn show_start_help(config: &AppConfig, subs_path: &Path) {
    println!("═════════════════ start – запуск прокси ═════════════════");
    println!("  vpn-manager start [menu]                это меню");
    println!("  vpn-manager start now                   последний профиль");
    println!("  vpn-manager start region <REGION>       регион (ru, eu, de…)");
    println!("  vpn-manager start sub <ID>              подписка по ID");
    println!("  vpn-manager start exec <cmd...>         выполнить в неймспейсе");
    println!("  Флаги:");
    println!("    -m, --method <random|fastest>        метод выбора (по умолчанию fastest)");
    println!("    -r, --rotate <S>                     интервал ротации (по умолчанию 300)");
    println!("    все флаги xray-knife proxy также принимаются");
    println!("Регионы:");
    // (список регионов с подсчётом живых)
    // ...
    println!("Подписки:");
    subs::list_subscriptions(subs_path, config);
    println!("════════════════════════════════════════════════════════");
}

pub fn get_current_config_info(config: &AppConfig) -> Option<ConfigInfo> {
    let cfg_file = if config.last_region.starts_with("_sub_") {
        let id = &config.last_region[5..];
        dirs::config_dir().unwrap().join("vpn-manager").join(format!("sub_{}_live.txt", id))
    } else {
        let region = utils::resolve_region(&config.last_region)?;
        dirs::config_dir().unwrap().join("vpn-manager").join("lists").join(format!("{}.txt", region))
    };
    if !cfg_file.exists() { return None; }
    let first_link = fs::read_to_string(cfg_file).ok()?.lines().next()?.to_string();
    let host = utils::extract_host(&first_link);
    if host == "unknown" { return None; }
    let ip = utils::resolve_ip(&host)?;
    let code = crate::geo::country_code(&ip, &config.geoip_db);
    let country = code.clone().unwrap_or_else(|| "??".into());
    let flag = match code.as_deref() {
        Some("RU") => "🇷🇺", Some("US") => "🇺🇸", Some("DE") => "🇩🇪",
        Some("PL") => "🇵🇱", Some("FI") => "🇫🇮", Some("NL") => "🇳🇱",
        Some("EU") => "🇪🇺", Some(_) => "🌍", _ => "🌍",
    };
    let protocol = if first_link.starts_with("vless://") { "vless" }
        else if first_link.starts_with("vmess://") { "vmess" }
        else if first_link.starts_with("trojan://") { "trojan" }
        else if first_link.starts_with("ss://") { "ss" }
        else { "?" };
    Some(ConfigInfo { flag: flag.to_string(), country, host, ip, protocol: protocol.to_string() })
}
