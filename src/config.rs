use anyhow::Result;
use serde::Deserialize;
use std::fs;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub watch: WatchConfig,
    pub extract: ExtractConfig,
    pub output: OutputConfig,
    pub history: HistoryConfig,
    pub retry: RetryConfig,
    pub startup: StartupConfig,
    pub notifications: NotificationConfig,
}

#[derive(Debug, Deserialize)]
pub struct WatchConfig {
    pub directory: String,
    pub stable_after: u64,
    pub allow_root_archives: bool,
}

#[derive(Debug, Deserialize)]
pub struct ExtractConfig {
    pub delete_archives: bool,
    pub dry_run: bool,
    pub keep_failed: bool,
}

#[derive(Debug, Deserialize)]
pub struct OutputConfig {
    pub directory: String,
}

#[derive(Debug, Deserialize)]
pub struct HistoryConfig {
    pub directory: String,
}

#[derive(Debug, Deserialize)]
pub struct RetryConfig {
    pub base_delay: u64,
    pub max_delay: u64,
}

#[derive(Debug, Deserialize)]
pub struct StartupConfig {
    pub scan_existing: bool,
}

#[derive(Debug, Deserialize, Clone)]
pub struct NotificationConfig {
    pub gotify: GotifyConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct GotifyConfig {
    pub enabled: bool,
    pub url: String,
    pub token: String,
    pub priority_success: i32,
    pub priority_error: i32,
}

impl Config {
    pub fn load(path: &str) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }
}
