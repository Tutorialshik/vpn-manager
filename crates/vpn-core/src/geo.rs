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
