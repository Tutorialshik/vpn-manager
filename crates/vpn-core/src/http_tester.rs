use crate::config::AppConfig;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct LiveResult {
    pub url: String,
    pub success: bool,
    pub live_file_path: Option<PathBuf>,
    pub error: Option<String>,
}

pub struct TestConfig<'a> {
    pub sub_id: usize,
    pub timeout: u64,
    pub threads: usize,
    pub insecure: bool,
    pub speedtest: bool,
    pub show_info: bool,
    pub log_dir: PathBuf,
    #[allow(dead_code)]
    pub config: &'a AppConfig,
}

pub trait HttpTester {
    fn run_tests(
        &self,
        config_file: &Path,
        urls: &[String],
        cfg: &TestConfig,
    ) -> anyhow::Result<Vec<LiveResult>>;
}
