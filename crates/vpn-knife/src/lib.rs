use anyhow::{bail, Context, Result};
use std::path::Path;
use std::process::{Child, Command};

/// Загрузить подписку через xray-knife subs fetch
pub fn fetch_subscription(url: &str, output: &Path) -> Result<()> {
    let status = Command::new("xray-knife")
        .args(["subs", "fetch", "-u", url, "-o", output.to_str().unwrap()])
        .status()
        .context("Не удалось запустить xray-knife subs fetch")?;
    if !status.success() {
        bail!(
            "xray-knife subs fetch завершился с ошибкой (код {:?})",
            status.code()
        );
    }
    Ok(())
}

/// Запустить HTTP-тест одного URL и сохранить живые конфиги в `live_file`.
/// Возвращает `true`, если файл с живыми конфигами был создан и не пуст.
#[allow(clippy::too_many_arguments)]
pub fn run_single_http_test(
    config_file: &Path,
    url: &str,
    timeout: u64,
    threads: usize,
    insecure: bool,
    speedtest: bool,
    show_info: bool,
    live_file: &Path,
    log_file: &Path,
) -> Result<bool> {
    let mut cmd = Command::new("xray-knife");
    cmd.arg("http")
        .arg("-f")
        .arg(config_file)
        .arg("-d")
        .arg(timeout.to_string())
        .arg("-t")
        .arg(threads.to_string())
        .arg("-u")
        .arg(url)
        .arg("-o")
        .arg(live_file);
    if insecure {
        cmd.arg("-e");
    }
    if speedtest {
        cmd.arg("--speedtest");
    }

    if show_info {
        cmd.stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit());
        let status = cmd.status()?;
        if !status.success() {
            bail!("xray-knife http завершился с ошибкой");
        }
    } else {
        let output = cmd.output()?;
        if let Ok(mut f) = std::fs::File::create(log_file) {
            use std::io::Write;
            f.write_all(&output.stdout).ok();
            f.write_all(&output.stderr).ok();
        }
    }

    if live_file.exists() {
        let meta = std::fs::metadata(live_file)?;
        if meta.len() > 0 {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Запустить прокси xray-knife и вернуть дочерний процесс.
pub fn spawn_proxy(args: &[String], log_file: &Path) -> Result<Child> {
    let log = std::fs::File::create(log_file)?;
    Command::new("xray-knife")
        .arg("proxy")
        .args(args)
        .stdout(std::process::Stdio::from(log.try_clone()?))
        .stderr(std::process::Stdio::from(log))
        .spawn()
        .context("Не удалось запустить xray-knife proxy")
}

/// Запустить произвольную команду xray-knife (например, cfscanner) с аргументами.
pub fn run_knife(cmd: &str, args: &[String]) -> Result<()> {
    let mut child = Command::new("xray-knife")
        .arg(cmd)
        .args(args)
        .spawn()
        .context("Не удалось запустить xray-knife")?;
    child.wait()?;
    Ok(())
}
