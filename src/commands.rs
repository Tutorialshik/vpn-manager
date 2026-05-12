use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct CommandsHelp {
    pub update: CommandHelp,
    pub start: CommandHelp,
    #[serde(rename = "_global_help")]
    pub global_help: GlobalHelp,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct CommandHelp {
    pub summary: String,
    pub usage: Option<String>,
    pub flags: Option<HashMap<String, String>>,
    pub regions: Option<HashMap<String, String>>,
}

#[derive(Debug, Deserialize)]
pub struct GlobalHelp {
    pub description: String,
    pub commands: HashMap<String, String>,
}

impl GlobalHelp {
    pub fn global_usage(&self) -> String {
        let mut out = format!("{}\n\n", self.description);
        for (cmd, desc) in &self.commands {
            if desc.is_empty() {
                // просто выводим заголовок-разделитель
                out.push_str(&format!("{}\n", cmd));
            } else {
                out.push_str(&format!("  {:12} - {}\n", cmd, desc));
            }
        }
        out
    }
}

pub fn load_commands(path: &Path) -> Result<CommandsHelp> {
    let content = fs::read_to_string(path)?;
    serde_json::from_str(&content).context("Ошибка парсинга commands.json")
}
