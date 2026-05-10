use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use crate::config::AppConfig;
use crate::utils;

pub fn is_running(config: &AppConfig) -> bool {
    let pid_path = pid_file_path(config);
    if let Ok(pid_str) = fs::read_to_string(&pid_path) {
        if let Ok(pid) = pid_str.trim().parse::<i32>() {
            // check if process exists
            unsafe { libc::kill(pid, 0) == 0 }
        } else { false }
    } else { false }
}

fn pid_file_path(config: &AppConfig) -> PathBuf {
    dirs::config_dir().unwrap().join("vpn-manager").join("xray-knife.pid")
}

pub fn stop_proxy(config: &AppConfig) -> Result<()> {
    let pid_path = pid_file_path(config);
    if let Ok(pid_str) = fs::read_to_string(&pid_path) {
        if let Ok(pid) = pid_str.trim().parse::<i32>() {
            if unsafe { libc::kill(pid, 0) } == 0 {
                unsafe { libc::kill(pid, 15) }; // SIGTERM
                std::thread::sleep(std::time::Duration::from_secs(1));
                let _ = unsafe { libc::kill(pid, 9) }; // SIGKILL
                println!("✅ Прокси остановлен (PID {})", pid);
            }
        }
        let _ = fs::remove_file(&pid_path);
    }
    // Убиваем все xray-knife proxy на всякий случай
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
                let live = dirs::config_dir().unwrap().join("vpn-manager").join(format!("sub_{}_live.txt", id));
                if !live.exists() {
                    anyhow::bail!("Сначала обновите подписку {}", id);
                }
                (live, config.last_region.clone())
            } else {
                let region = utils::resolve_region(&config.last_region).unwrap_or_else(|| "eu".into());
                let file = dirs::config_dir().unwrap().join("vpn-manager").join("lists").join(format!("{}.txt", region));
                if !file.exists() {
                    anyhow::bail!("Файл региона {} не найден", region);
                }
                (file, region)
            }
        }
        region if utils::resolve_region(region).is_some() => {
            let r = utils::resolve_region(region).unwrap();
            let file = dirs::config_dir().unwrap().join("vpn-manager").join("lists").join(format!("{}.txt", r));
            if !file.exists() || fs::metadata(&file)?.len() == 0 {
                anyhow::bail!("Файл региона {} не найден или пуст", r);
            }
            (file, r)
        }
        other if other.starts_with("sub") => {
            // sub <ID>
            let id_str = target.split_whitespace().nth(1).context("Укажите ID")?;
            let id: usize = id_str.parse()?;
            let live = dirs::config_dir().unwrap().join("vpn-manager").join(format!("sub_{}_live.txt", id));
            if !live.exists() {
                anyhow::bail!("Сначала update {}", id);
            }
            (live, format!("_sub_{}", id))
        }
        _ => anyhow::bail!("Неизвестная цель: {}. Используйте now, регион или sub ID", target),
    };

    // Формируем аргументы для xray-knife proxy
    let mut args = vec![
        "-f".to_string(), cfg_file.to_string_lossy().into(),
    ];
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
    args.push("--rotate".into()); args.push(config.rotate.to_string());
    args.push("--blacklist-duration".into()); args.push(config.blacklist_duration.to_string());
    args.push("--blacklist-strikes".into()); args.push(config.blacklist_strikes.to_string());
    // Пользовательские флаги
    args.extend_from_slice(extra_args);

    let log_file_path = dirs::config_dir().unwrap().join("vpn-manager").join("vpn-manager.log");
    let log_file = fs::File::create(&log_file_path)?;
    let mut child = Command::new("xray-knife")
        .arg("proxy")
        .args(&args)
        .stdout(Stdio::from(log_file.try_clone()?))
        .stderr(Stdio::from(log_file))
        .spawn()
        .context("Не удалось запустить xray-knife")?;

    let pid = child.id();
    fs::write(pid_file_path(config), pid.to_string())?;

    config.last_region = save_region;
    config.save(config_path)?;

    println!("✅ Прокси запущен (PID {})", pid);
    Ok(())
}
