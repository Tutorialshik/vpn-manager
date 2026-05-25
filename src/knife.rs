use crate::l10n;
use crate::t;
use anyhow::{bail, Context, Result};
use std::path::Path;
use std::process::{Child, Command};

#[allow(dead_code)]
pub fn fetch_subscription(url: &str, output: &Path) -> Result<()> {
    let status = Command::new("xray-knife")
        .args(["subs", "fetch", "-u", url, "-o", output.to_str().unwrap()])
        .status()
        .context(t!("knife.fetch_error"))?;
    if !status.success() {
        bail!(l10n::t_fmt(
            "knife.fetch_failed",
            &[&status.code().unwrap_or(1).to_string()]
        ));
    }
    Ok(())
}

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
            bail!(t!("knife.http_error"));
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

pub fn spawn_proxy(args: &[String], log_file: &Path) -> Result<Child> {
    let log = std::fs::File::create(log_file)?;
    Command::new("xray-knife")
        .arg("proxy")
        .args(args)
        .stdout(std::process::Stdio::from(log.try_clone()?))
        .stderr(std::process::Stdio::from(log))
        .spawn()
        .context(t!("knife.proxy_start_fail"))
}

pub fn run_knife(cmd: &str, args: &[String]) -> Result<()> {
    let mut child = Command::new("xray-knife")
        .arg(cmd)
        .args(args)
        .spawn()
        .context(t!("knife.knife_start_fail"))?;
    child.wait()?;
    Ok(())
}
