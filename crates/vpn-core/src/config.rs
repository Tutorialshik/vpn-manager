use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct AppConfig {
    pub default_port: u16,
    pub listen_ip: String,
    pub http_test_timeout: u64,
    pub http_test_threads: usize,
    pub parallel_tests: usize,
    pub http_url_pool_data: String,
    pub http_url_active_ids: String,
    pub update_protocol_filter: String,
    pub update_limit_per_sub: usize,
    pub keep_raw: bool,
    pub auto_update_interval: u64,
    pub auto_update_ids: String,
    pub menu_update_interval: u64,
    pub select_mode: String,
    pub menu_position: usize,
    pub geoip_db: String,
    pub log_file: String,
    pub http_log_dir: String,
    pub last_region: String,
    pub last_mode_type: String,
    pub last_inbound_proto: String,
    pub core: String,
    pub insecure: bool,
    pub rotate: u64,
    pub blacklist_duration: u64,
    pub blacklist_strikes: u32,
    pub speedtest: bool,
    pub http_verbose: bool,
    pub show_update_info: bool,
    pub auto_menu_update_enabled: bool,
    pub auto_menu_update_interval: u64,

    pub parallel_enabled: bool,
    pub parallel_geo_balancing: bool,
    pub parallel_geo_cache_ttl: u64,
    pub parallel_max_profiles: usize,
    pub parallel_timeout_ms: u64,
    pub parallel_strategy: String,
    pub parallel_db_path: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("vpn-manager");
        Self {
            default_port: 8880,
            listen_ip: "127.0.0.1".into(),
            http_test_timeout: 5000,
            http_test_threads: 50,
            parallel_tests: 10,
            http_url_pool_data: "1|https://cloudflare.com/cdn-cgi/trace;2|https://www.google.com/generate_204;3|https://httpbin.org/ip".into(),
            http_url_active_ids: "1,2,3".into(),
            update_protocol_filter: "all".into(),
            update_limit_per_sub: 0,
            keep_raw: true,
            auto_update_interval: 0,
            auto_update_ids: "all".into(),
            menu_update_interval: 0,
            select_mode: "fastest".into(),
            menu_position: 1,
            geoip_db: "/usr/share/GeoIP/GeoLite2-Country.mmdb".into(),
            log_file: config_dir.join("vpn-manager.log").to_string_lossy().into(),
            http_log_dir: config_dir.join("http-logs").to_string_lossy().into(),
            last_region: "eu".into(),
            last_mode_type: "inbound".into(),
            last_inbound_proto: "socks".into(),
            core: "xray".into(),
            insecure: false,
            rotate: 300,
            blacklist_duration: 600,
            blacklist_strikes: 3,
            speedtest: false,
            http_verbose: true,
            show_update_info: false,
            auto_menu_update_enabled: false,
            auto_menu_update_interval: 0,
            parallel_enabled: false,
            parallel_geo_balancing: false,
            parallel_geo_cache_ttl: 3600,
            parallel_max_profiles: 3,
            parallel_timeout_ms: 5000,
            parallel_strategy: "latency".into(),
            parallel_db_path: config_dir.join("parallel_stats.db").to_string_lossy().into(),
        }
    }
}

impl AppConfig {
    pub fn load_or_default(path: &Path) -> anyhow::Result<Self> {
        if path.exists() {
            let content = fs::read_to_string(path)?;
            serde_json::from_str(&content).context("Ошибка парсинга config.json")
        } else {
            let cfg = Self::default();
            let json = serde_json::to_string_pretty(&cfg)?;
            fs::write(path, json)?;
            Ok(cfg)
        }
    }

    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, json)?;
        Ok(())
    }
}
