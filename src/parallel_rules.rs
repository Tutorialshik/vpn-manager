use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ParallelRule {
    pub domain: String,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    pub max_profiles: Option<usize>,
    pub strategy: Option<String>,
    pub exit_countries: Option<Vec<String>>,
    #[serde(default)]
    pub allow_fallback: bool,
}

#[allow(dead_code)]
fn default_enabled() -> bool {
    true
}

#[allow(dead_code)]
pub fn load_rules(path: &Path) -> Result<Vec<ParallelRule>> {
    if !path.exists() {
        return Ok(vec![]);
    }
    let content = fs::read_to_string(path)?;
    let rules: Vec<ParallelRule> =
        serde_json::from_str(&content).context("Ошибка парсинга parallel_rules.json")?;
    Ok(rules)
}
