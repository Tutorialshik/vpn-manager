use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Subscription {
    pub id: usize,
    pub name: String,
    pub url: String,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_enabled() -> bool {
    true
}

pub type Subscriptions = Vec<Subscription>;

#[derive(Debug, Clone)]
pub struct ConfigInfo {
    pub flag: String,
    pub country: String,
    pub host: String,
    pub ip: String,
    pub protocol: String,
}

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

#[derive(Debug, Clone, Copy, clap::Subcommand)]
pub enum ChangeCmd {
    Next,
    Prev,
    Random,
    Fastest,
}
