use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use crate::config::AppConfig;
use crate::subs;
use crate::utils;

pub fn is_running(config: &AppConfig) -> bool {
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

pub fn stop_proxy(config: &AppConfig) -> Result<()> {
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
    subs_path: &Path,
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
                anyhow::bail!(
                    "Неизвестная цель: {}. Используйте now, регион или sub ID",
                    target
                );
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
    if config.insecure {
        args.push("-e".into());
    }
    args.push("--core".into());
    args.push(config.core.clone());
    args.push("--rotate".into());
    args.push(config.rotate.to_string());
    args.push("--blacklist-duration".into());
    args.push(config.blacklist_duration.to_string());
    args.push("--blacklist-strikes".into());
    args.push(config.blacklist_strikes.to_string());
    args.extend_from_slice(extra_args);

    let log_file_path = dirs::config_dir()
        .unwrap()
        .join("vpn-manager")
        .join("vpn-manager.log");
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
    println!("  vpn-manager start now                последний профиль");
    println!("  vpn-manager start <region>           регион (ru, eu, de…)");
    println!("  vpn-manager start sub <ID>           подписка по ID");
    println!();
    println!("Регионы:");
    let regions = [
        ("ru", "Россия", "🇷🇺"),
        ("us", "США", "🇺🇸"),
        ("eu", "Европа", "🇪🇺"),
        ("de", "Германия", "🇩🇪"),
        ("pl", "Польша", "🇵🇱"),
        ("fi", "Финляндия", "🇫🇮"),
        ("nl", "Нидерланды", "🇳🇱"),
        ("other", "Остальные", "🌍"),
    ];
    let lists_dir = dirs::config_dir().unwrap().join("vpn-manager").join("lists");
    for (code, name, flag) in &regions {
        let file_path = lists_dir.join(format!("{}.txt", code));
        let count = if file_path.exists() {
            std::fs::read_to_string(&file_path)
                .map(|s| s.lines().count())
                .unwrap_or(0)
        } else {
            0
        };
        println!("  {} {:<6} – {:20} {:6} живых", flag, code, name, count);
    }
    println!();
    println!("Подписки:");
    subs::list_subscriptions(subs_path, config);
    println!("════════════════════════════════════════════════════════");
}
