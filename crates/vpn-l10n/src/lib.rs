use anyhow::{Context, Result};
use once_cell::sync::Lazy;
use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

static TRANSLATIONS: Lazy<Mutex<Option<Value>>> = Lazy::new(|| Mutex::new(None));

pub fn init(locale_path: &PathBuf) -> Result<()> {
    let content =
        fs::read_to_string(locale_path).context("Не удалось прочитать файл локализации")?;
    let v: Value = serde_json::from_str(&content).context("Ошибка парсинга локализации")?;
    let mut store = match TRANSLATIONS.lock() {
        Ok(guard) => guard,
        Err(poisoned) => {
            eprintln!("Восстановление после отравления мьютекса локализации");
            poisoned.into_inner()
        }
    };
    *store = Some(v);
    Ok(())
}

pub fn t(key: &str) -> String {
    let store = match TRANSLATIONS.lock() {
        Ok(guard) => guard,
        Err(poisoned) => {
            eprintln!("Мьютекс локализации отравлен, возвращаем ключ");
            poisoned.into_inner()
        }
    };
    let val = store.as_ref().and_then(|v| {
        let parts: Vec<&str> = key.split('.').collect();
        let mut current = v;
        for part in parts {
            match current.get(part) {
                Some(v) => current = v,
                None => return None,
            }
        }
        current.as_str().map(|s| s.to_owned())
    });
    val.unwrap_or_else(|| key.to_string())
}

pub fn t_fmt(key: &str, args: &[&str]) -> String {
    let template = t(key);
    let mut result = template;
    for (i, arg) in args.iter().enumerate() {
        result = result.replace(&format!("{{{}}}", i), arg);
    }
    result
}
