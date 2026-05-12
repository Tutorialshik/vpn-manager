use std::net::ToSocketAddrs;
use std::process::Command;

pub fn country_code(ip: &str, db_path: &str) -> Option<String> {
    let output = Command::new("mmdblookup")
        .args(["--file", db_path, "--ip", ip, "country", "iso_code"])
        .output()
        .ok()?;
    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        stdout
            .lines()
            .find(|l| l.contains("utf8_string"))
            .and_then(|l| l.split('"').nth(3).map(|s| s.to_string()))
    } else {
        None
    }
}

#[allow(dead_code)]
pub fn country_name(ip: &str, db_path: &str) -> Option<String> {
    let output = Command::new("mmdblookup")
        .args(["--file", db_path, "--ip", ip, "country", "names", "en"])
        .output()
        .ok()?;
    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        stdout
            .lines()
            .find(|l| l.contains("utf8_string"))
            .and_then(|l| l.split('"').nth(3).map(|s| s.to_string()))
    } else {
        None
    }
}

#[allow(dead_code)]
pub fn get_host_country(
    host: &str,
    conn: &rusqlite::Connection,
    geoip_db_path: &str,
    cache_ttl: u64,
) -> Option<String> {
    if let Some(cached) = crate::db::get_cached_country(conn, host, cache_ttl) {
        return Some(cached);
    }

    let ip = resolve_host_to_ip(host)?;
    let country = country_code(&ip, geoip_db_path)?;
    crate::db::cache_host_country(conn, host, &country).ok();
    Some(country)
}

#[allow(dead_code)]
fn resolve_host_to_ip(host: &str) -> Option<String> {
    (host, 0)
        .to_socket_addrs()
        .ok()?
        .next()
        .map(|addr| addr.ip().to_string())
}
