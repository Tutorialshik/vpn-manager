use anyhow::{bail, Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use vpn_core::config::AppConfig;
use vpn_core::http_tester::{HttpTester, TestConfig};
use vpn_core::types::Subscription;
use vpn_core::utils;

pub fn update_single_sub(
    sub: &Subscription,
    proto: &str,
    limit: usize,
    _keep_raw: bool,
    show_info: bool,
    config: &AppConfig,
    tester: &dyn HttpTester,
) -> Result<Vec<PathBuf>> {
    println!("ℹ️ Обновление [{}] {}", sub.id, sub.name);
    let config_dir = dirs::config_dir()
        .context("Не удалось определить config директорию")?
        .join("vpn-manager");
    std::fs::create_dir_all(&config_dir)?;

    // Загрузка подписки (пока синхронно, можно будет заменить на асинхронную)
    let raw_path = format!("/tmp/vpn-sub-{}-raw.txt", sub.id);
    if sub.url.starts_with("http://") || sub.url.starts_with("https://") {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_millis(config.http_test_timeout))
            .build()
            .context("Не удалось создать HTTP-клиент")?;
        let resp = client.get(&sub.url).send()?;
        if !resp.status().is_success() {
            bail!("Ошибка загрузки подписки: HTTP {}", resp.status());
        }
        let body = resp.text()?;
        fs::write(&raw_path, body)?;
    } else if sub.url.starts_with("file://") {
        fs::copy(sub.url.trim_start_matches("file://"), &raw_path)?;
    } else if Path::new(&sub.url).exists() {
        fs::copy(&sub.url, &raw_path)?;
    } else {
        bail!("Неизвестный источник: {}", sub.url);
    }
    let content = fs::read_to_string(&raw_path)?.replace('\r', "");
    fs::write(&raw_path, content)?;

    // Фильтрация
    let filtered_path = format!("/tmp/vpn-sub-{}-filtered.txt", sub.id);
    utils::filter_subscription_file(&raw_path, &filtered_path, proto, limit)?;

    // HTTP тесты
    let active_urls = utils::get_active_urls(config);
    if active_urls.is_empty() {
        bail!("Нет активных тестовых URL");
    }

    let test_config = TestConfig {
        sub_id: sub.id,
        timeout: config.http_test_timeout,
        threads: config.http_test_threads,
        insecure: config.insecure,
        speedtest: config.speedtest,
        show_info,
        log_dir: PathBuf::from(&config.http_log_dir),
        config,
    };

    let results = tester.run_tests(Path::new(&filtered_path), &active_urls, &test_config)?;

    let live_files: Vec<PathBuf> = results
        .into_iter()
        .filter_map(|r| {
            if r.success {
                r.live_file_path
            } else {
                if let Some(err) = r.error {
                    eprintln!("⚠️ Ошибка HTTP теста для {}: {}", r.url, err);
                }
                None
            }
        })
        .collect();

    if live_files.is_empty() {
        bail!("Нет живых конфигов");
    }

    // Не удаляем временные файлы, если keep_raw? Но здесь мы уже возвращаем только live_files.
    // keep_raw учитывается вызывающей стороной.
    Ok(live_files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use vpn_core::http_tester::{HttpTester, LiveResult, TestConfig};
    use vpn_core::types::Subscription;

    struct MockHttpTester {
        results: Vec<LiveResult>,
    }

    impl MockHttpTester {
        fn new(results: Vec<LiveResult>) -> Self {
            MockHttpTester { results }
        }
    }

    impl HttpTester for MockHttpTester {
        fn run_tests(
            &self,
            _config_file: &Path,
            _urls: &[String],
            _cfg: &TestConfig,
        ) -> anyhow::Result<Vec<LiveResult>> {
            Ok(self.results.clone())
        }
    }

    /// Проверяем, что update_single_sub возвращает только успешные live_file_path
    #[test]
    fn test_update_single_sub_with_mock_tester() -> Result<()> {
        // Создаём подписку с file://, чтобы избежать реальной сети
        let dir = tempfile::tempdir()?;
        let sub_file = dir.path().join("sub.txt");
        std::fs::write(
            &sub_file,
            "vless://example.com:443\nvmess://example.com:100\n",
        )?;

        let sub = Subscription {
            id: 1,
            name: "test".into(),
            url: format!("file://{}", sub_file.display()),
            enabled: true,
        };

        let mut config = vpn_core::config::AppConfig::default();
        config.http_log_dir = dir.path().to_string_lossy().to_string();

        // Мок-тестер: первый URL успешен, второй неудачен
        let live_path = dir.path().join("live1.txt");
        std::fs::write(&live_path, "live config")?;
        let mock = MockHttpTester::new(vec![
            LiveResult {
                url: "https://a.com".into(),
                success: true,
                live_file_path: Some(live_path.clone()),
                error: None,
            },
            LiveResult {
                url: "https://b.com".into(),
                success: false,
                live_file_path: None,
                error: Some("timeout".into()),
            },
        ]);

        let result = update_single_sub(&sub, "all", 0, false, false, &config, &mock)?;

        // Должен вернуть только один успешный путь
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], live_path);

        Ok(())
    }
}
