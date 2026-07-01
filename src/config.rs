use anyhow::Result;
use serde::Deserialize;
use std::fs;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub watch: WatchConfig,
    pub extract: ExtractConfig,
}

#[derive(Debug, Deserialize)]
pub struct WatchConfig {
    pub directory: String,
    pub stable_after: u64,
}

#[derive(Debug, Deserialize)]
pub struct ExtractConfig {
    pub delete_archives: bool,
    pub keep_failed: bool,
}

impl Config {
    pub fn load(path: &str) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }
}
