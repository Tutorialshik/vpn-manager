use rusqlite::Connection;
use std::path::Path;
use vpn_core::config::AppConfig;
use vpn_core::http_tester::HttpTester;

pub struct UpdateParams<'a> {
    pub target: &'a str,
    pub protocol: &'a str,
    pub limit: usize,
    pub keep_raw: bool,
    pub show_info: bool,
    pub config: &'a AppConfig,
    pub subs_path: &'a Path,
    pub db: Option<&'a Connection>,
    pub tester: &'a dyn HttpTester,
}

mod classifier;
mod coordinator;
mod stats;
mod utils;

pub use coordinator::handle_update;
