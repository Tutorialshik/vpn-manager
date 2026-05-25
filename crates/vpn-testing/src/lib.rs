use anyhow::Result;
use std::path::{Path, PathBuf};
use vpn_core::http_tester::{HttpTester, LiveResult, TestConfig};
// use vpn_knife;

pub struct XrayKnifeHttpTester;

impl HttpTester for XrayKnifeHttpTester {
    fn run_tests(
        &self,
        config_file: &Path,
        urls: &[String],
        cfg: &TestConfig,
    ) -> Result<Vec<LiveResult>> {
        let mut handles = vec![];
        let config_file = config_file.to_path_buf();

        for url in urls {
            let url = url.clone();
            let cfg_file = config_file.clone();
            let timeout = cfg.timeout;
            let threads = cfg.threads;
            let insecure = cfg.insecure;
            let speedtest = cfg.speedtest;
            let show_info = cfg.show_info;
            let log_path = cfg.log_dir.join(format!(
                "vpn-http-{}-{}.log",
                cfg.sub_id,
                sanitize_filename(&url)
            ));
            let sub_id = cfg.sub_id;

            let live_file = tempfile::NamedTempFile::new()?.into_temp_path();
            let live_path = live_file.to_path_buf();

            let handle = std::thread::spawn(move || -> LiveResult {
                let success = vpn_knife::run_single_http_test(
                    &cfg_file, &url, timeout, threads, insecure, speedtest, show_info, &live_path,
                    &log_path,
                );

                match success {
                    Ok(true) => {
                        let perm_path = PathBuf::from(format!(
                            "/tmp/vpn-sub-{}-live-{}.txt",
                            sub_id,
                            sanitize_filename(&url)
                        ));
                        if let Err(e) = std::fs::rename(&live_path, &perm_path) {
                            LiveResult {
                                url,
                                success: false,
                                live_file_path: None,
                                error: Some(format!("Ошибка перемещения файла: {}", e)),
                            }
                        } else {
                            LiveResult {
                                url,
                                success: true,
                                live_file_path: Some(perm_path),
                                error: None,
                            }
                        }
                    }
                    Ok(false) => LiveResult {
                        url,
                        success: false,
                        live_file_path: None,
                        error: Some("Пустой результат теста".into()),
                    },
                    Err(e) => LiveResult {
                        url,
                        success: false,
                        live_file_path: None,
                        error: Some(e.to_string()),
                    },
                }
            });
            handles.push(handle);
        }

        let mut results = vec![];
        for h in handles {
            match h.join() {
                Ok(res) => results.push(res),
                Err(panic) => {
                    results.push(LiveResult {
                        url: "unknown".into(),
                        success: false,
                        live_file_path: None,
                        error: Some(format!("Поток HTTP-теста упал: {:?}", panic)),
                    });
                }
            }
        }
        Ok(results)
    }
}

fn sanitize_filename(s: &str) -> String {
    s.replace(['/', ':', '?', '&'], "_")
}
