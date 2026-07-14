use anyhow::{Context, Result};
use serde::Deserialize;
use std::{fs, path::Path};

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub watch: WatchConfig,

    #[serde(default)]
    pub extract: ExtractConfig,

    #[serde(default)]
    pub output: OutputConfig,

    #[serde(default)]
    pub history: HistoryConfig,

    #[serde(default)]
    pub retry: RetryConfig,

    #[serde(default)]
    pub startup: StartupConfig,

    #[serde(default)]
    pub notifications: NotificationConfig,

    #[serde(default)]
    pub web: WebConfig,
}

impl Config {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        load_config(path.as_ref())
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct WatchConfig {
    #[serde(default)]
    pub directory: String,

    #[serde(default)]
    pub directories: Vec<String>,

    #[serde(default = "default_stable_after")]
    pub stable_after: u64,

    #[serde(default)]
    pub allow_root_archives: bool,
}

impl WatchConfig {
    pub fn resolved_directories(&self) -> Vec<&str> {
        let mut directories = Vec::new();

        for directory in std::iter::once(self.directory.as_str())
            .chain(self.directories.iter().map(String::as_str))
        {
            let directory = directory.trim();

            if directory.is_empty() {
                continue;
            }

            if directories.contains(&directory) {
                continue;
            }

            directories.push(directory);
        }

        directories
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct ExtractConfig {
    #[serde(default = "default_delete_archives")]
    pub delete_archives: bool,

    #[serde(default = "default_keep_failed")]
    pub keep_failed: bool,

    #[serde(default)]
    pub password_file: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct OutputConfig {
    #[serde(default = "default_output_directory")]
    pub directory: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct HistoryConfig {
    #[serde(default = "default_history_directory")]
    pub directory: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RetryConfig {
    #[serde(default = "default_retry_base_delay")]
    pub base_delay: u64,

    #[serde(default = "default_retry_max_delay")]
    pub max_delay: u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct StartupConfig {
    #[serde(default)]
    pub scan_existing: bool,
}

#[derive(Debug, Deserialize, Clone)]
pub struct WebConfig {
    #[serde(default)]
    pub enabled: bool,

    #[serde(default = "default_web_bind")]
    pub bind: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct NotificationConfig {
    #[serde(default)]
    pub enabled: bool,

    #[serde(default = "default_notification_provider")]
    pub provider: String,

    #[serde(default)]
    pub ntfy: NtfyConfig,

    // Legacy v1.0.x configuration. Kept during the staged migration so the
    // application remains buildable and existing installations keep parsing.
    #[serde(default)]
    pub gotify: GotifyConfig,
}

#[derive(Deserialize, Clone)]
pub struct NtfyConfig {
    #[serde(default)]
    pub server: String,

    #[serde(default)]
    pub topic: String,

    #[serde(default)]
    pub token: String,

    #[serde(default = "default_ntfy_priority_success")]
    pub priority_success: u8,

    #[serde(default = "default_ntfy_priority_error")]
    pub priority_error: u8,

    #[serde(default = "default_notify_on_success")]
    pub notify_on_success: bool,

    #[serde(default = "default_notify_on_error")]
    pub notify_on_error: bool,

    #[serde(default)]
    pub notify_on_every_error: bool,

    #[serde(default = "default_notify_after_attempts")]
    pub notify_after_attempts: u64,
}

#[derive(Deserialize, Clone)]
pub struct GotifyConfig {
    #[serde(default)]
    pub enabled: bool,

    #[serde(default)]
    pub url: String,

    #[serde(default)]
    pub token: String,

    #[serde(default = "default_gotify_priority_success")]
    pub priority_success: i32,

    #[serde(default = "default_gotify_priority_error")]
    pub priority_error: i32,

    #[serde(default = "default_notify_on_success")]
    pub notify_on_success: bool,

    #[serde(default = "default_notify_on_error")]
    pub notify_on_error: bool,

    #[serde(default)]
    pub notify_on_every_error: bool,

    #[serde(default = "default_notify_after_attempts")]
    pub notify_after_attempts: u64,
}

impl Default for ExtractConfig {
    fn default() -> Self {
        Self {
            delete_archives: default_delete_archives(),
            keep_failed: default_keep_failed(),
            password_file: String::new(),
        }
    }
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            directory: default_output_directory(),
        }
    }
}

impl Default for HistoryConfig {
    fn default() -> Self {
        Self {
            directory: default_history_directory(),
        }
    }
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            base_delay: default_retry_base_delay(),
            max_delay: default_retry_max_delay(),
        }
    }
}

impl Default for StartupConfig {
    fn default() -> Self {
        Self {
            scan_existing: false,
        }
    }
}

impl Default for WebConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            bind: default_web_bind(),
        }
    }
}

impl Default for NotificationConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            provider: default_notification_provider(),
            ntfy: NtfyConfig::default(),
            gotify: GotifyConfig::default(),
        }
    }
}

impl std::fmt::Debug for NtfyConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let token_display = if self.token.trim().is_empty() {
            "<empty>"
        } else {
            "<redacted>"
        };

        f.debug_struct("NtfyConfig")
            .field("server", &self.server)
            .field("topic", &self.topic)
            .field("token", &token_display)
            .field("priority_success", &self.priority_success)
            .field("priority_error", &self.priority_error)
            .field("notify_on_success", &self.notify_on_success)
            .field("notify_on_error", &self.notify_on_error)
            .field("notify_on_every_error", &self.notify_on_every_error)
            .field("notify_after_attempts", &self.notify_after_attempts)
            .finish()
    }
}

impl Default for NtfyConfig {
    fn default() -> Self {
        Self {
            server: String::new(),
            topic: String::new(),
            token: String::new(),
            priority_success: default_ntfy_priority_success(),
            priority_error: default_ntfy_priority_error(),
            notify_on_success: default_notify_on_success(),
            notify_on_error: default_notify_on_error(),
            notify_on_every_error: false,
            notify_after_attempts: default_notify_after_attempts(),
        }
    }
}

impl std::fmt::Debug for GotifyConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let token_display = if self.token.trim().is_empty() {
            "<empty>"
        } else {
            "<redacted>"
        };

        f.debug_struct("GotifyConfig")
            .field("enabled", &self.enabled)
            .field("url", &self.url)
            .field("token", &token_display)
            .field("priority_success", &self.priority_success)
            .field("priority_error", &self.priority_error)
            .field("notify_on_success", &self.notify_on_success)
            .field("notify_on_error", &self.notify_on_error)
            .field("notify_on_every_error", &self.notify_on_every_error)
            .field("notify_after_attempts", &self.notify_after_attempts)
            .finish()
    }
}

impl Default for GotifyConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            url: String::new(),
            token: String::new(),
            priority_success: default_gotify_priority_success(),
            priority_error: default_gotify_priority_error(),
            notify_on_success: default_notify_on_success(),
            notify_on_error: default_notify_on_error(),
            notify_on_every_error: false,
            notify_after_attempts: default_notify_after_attempts(),
        }
    }
}

pub fn load_config(path: &Path) -> Result<Config> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Konnte Config nicht lesen: {}", path.display()))?;

    let config: Config = toml::from_str(&content)
        .with_context(|| format!("Konnte Config nicht parsen: {}", path.display()))?;

    validate_config(&config)?;

    Ok(config)
}

fn validate_config(config: &Config) -> Result<()> {
    if config.watch.resolved_directories().is_empty() {
        anyhow::bail!(
            "Config ungültig: watch.directory oder watch.directories muss mindestens einen Ordner enthalten"
        );
    }

    if config.watch.stable_after == 0 {
        anyhow::bail!("Config ungültig: watch.stable_after muss größer als 0 sein");
    }

    if config.output.directory.trim().is_empty() {
        anyhow::bail!("Config ungültig: output.directory darf nicht leer sein");
    }

    if config.history.directory.trim().is_empty() {
        anyhow::bail!("Config ungültig: history.directory darf nicht leer sein");
    }

    if config.retry.base_delay == 0 {
        anyhow::bail!("Config ungültig: retry.base_delay muss größer als 0 sein");
    }

    if config.retry.max_delay < config.retry.base_delay {
        anyhow::bail!(
            "Config ungültig: retry.max_delay darf nicht kleiner als retry.base_delay sein"
        );
    }

    if config.notifications.enabled {
        if config.notifications.provider.trim() != "ntfy" {
            anyhow::bail!("Config ungültig: notifications.provider muss für v1.1.0 'ntfy' sein");
        }

        let ntfy = &config.notifications.ntfy;
        let server = ntfy.server.trim();
        let topic = ntfy.topic.trim();

        if server.is_empty() {
            anyhow::bail!("Config ungültig: ntfy ist aktiviert, aber server ist leer");
        }

        if !server.starts_with("http://") && !server.starts_with("https://") {
            anyhow::bail!(
                "Config ungültig: notifications.ntfy.server muss mit http:// oder https:// beginnen"
            );
        }

        if topic.is_empty() {
            anyhow::bail!("Config ungültig: ntfy ist aktiviert, aber topic ist leer");
        }

        if topic.chars().any(char::is_whitespace) || topic.contains(['?', '#']) {
            anyhow::bail!(
                "Config ungültig: notifications.ntfy.topic darf keine Leerzeichen, '?' oder '#' enthalten"
            );
        }

        if !(1..=5).contains(&ntfy.priority_success) {
            anyhow::bail!(
                "Config ungültig: notifications.ntfy.priority_success muss zwischen 1 und 5 liegen"
            );
        }

        if !(1..=5).contains(&ntfy.priority_error) {
            anyhow::bail!(
                "Config ungültig: notifications.ntfy.priority_error muss zwischen 1 und 5 liegen"
            );
        }

        if ntfy.notify_after_attempts == 0 {
            anyhow::bail!(
                "Config ungültig: notifications.ntfy.notify_after_attempts muss mindestens 1 sein"
            );
        }
    }

    if config.notifications.gotify.enabled {
        if config.notifications.gotify.url.trim().is_empty() {
            anyhow::bail!("Config ungültig: Gotify ist aktiviert, aber url ist leer");
        }

        if config.notifications.gotify.token.trim().is_empty() {
            anyhow::bail!("Config ungültig: Gotify ist aktiviert, aber token ist leer");
        }
    }

    Ok(())
}

fn default_stable_after() -> u64 {
    30
}

fn default_delete_archives() -> bool {
    true
}

fn default_keep_failed() -> bool {
    true
}

fn default_output_directory() -> String {
    "/downloads/_extracted".to_string()
}

fn default_history_directory() -> String {
    "/state/history".to_string()
}

fn default_retry_base_delay() -> u64 {
    60
}

fn default_retry_max_delay() -> u64 {
    1800
}

fn default_notification_provider() -> String {
    "ntfy".to_string()
}

fn default_ntfy_priority_success() -> u8 {
    3
}

fn default_ntfy_priority_error() -> u8 {
    5
}

fn default_gotify_priority_success() -> i32 {
    3
}

fn default_gotify_priority_error() -> i32 {
    8
}

fn default_notify_on_success() -> bool {
    true
}

fn default_notify_on_error() -> bool {
    true
}

fn default_notify_after_attempts() -> u64 {
    3
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn loads_minimal_config_with_defaults() {
        let dir = tempdir().expect("tempdir");
        let config_file = dir.path().join("config.toml");

        fs::write(
            &config_file,
            r#"
[watch]
directory="/downloads"
"#,
        )
        .expect("write");

        let config = load_config(&config_file).expect("load config");

        assert_eq!(config.watch.directory, "/downloads");
        assert!(config.watch.directories.is_empty());
        assert_eq!(config.watch.resolved_directories(), vec!["/downloads"]);
        assert_eq!(config.watch.stable_after, 30);
        assert!(config.extract.delete_archives);
        assert!(config.extract.keep_failed);
        assert_eq!(config.output.directory, "/downloads/_extracted");
        assert_eq!(config.history.directory, "/state/history");
        assert_eq!(config.retry.base_delay, 60);
        assert_eq!(config.retry.max_delay, 1800);
        assert!(!config.notifications.enabled);
        assert_eq!(config.notifications.provider, "ntfy");
        assert!(!config.notifications.gotify.enabled);
    }

    #[test]
    fn combines_legacy_and_additional_watch_directories() {
        let dir = tempdir().expect("tempdir");

        let config_file = dir.path().join("config.toml");

        fs::write(
            &config_file,
            r#"
[watch]
directory="/downloads"

directories=[
    "/downloads2",
    " /downloads3 ",
    "/downloads"
]
"#,
        )
        .expect("write");

        let config = load_config(&config_file).expect("load config");

        assert_eq!(
            config.watch.resolved_directories(),
            vec!["/downloads", "/downloads2", "/downloads3",]
        );
    }

    #[test]
    fn accepts_watch_directories_without_legacy_directory() {
        let dir = tempdir().expect("tempdir");

        let config_file = dir.path().join("config.toml");

        fs::write(
            &config_file,
            r#"
[watch]
directories=[
    "/downloads-a",
    "/downloads-b"
]
"#,
        )
        .expect("write");

        let config = load_config(&config_file).expect("load config");

        assert!(config.watch.directory.is_empty());

        assert_eq!(
            config.watch.resolved_directories(),
            vec!["/downloads-a", "/downloads-b",]
        );
    }

    #[test]
    fn rejects_empty_watch_directory() {
        let dir = tempdir().expect("tempdir");
        let config_file = dir.path().join("config.toml");

        fs::write(
            &config_file,
            r#"
[watch]
directory=""
"#,
        )
        .expect("write");

        let err = load_config(&config_file).expect_err("should fail");
        assert!(format!("{:?}", err).contains("watch.directory"));
    }

    #[test]
    fn rejects_enabled_gotify_without_token() {
        let dir = tempdir().expect("tempdir");
        let config_file = dir.path().join("config.toml");

        fs::write(
            &config_file,
            r#"
[watch]
directory="/downloads"

[notifications.gotify]
enabled=true
url="https://gotify.example.com"
token=""
"#,
        )
        .expect("write");

        let err = load_config(&config_file).expect_err("should fail");
        assert!(format!("{:?}", err).contains("token"));
    }

    #[test]
    fn accepts_enabled_ntfy_without_token() {
        let dir = tempdir().expect("tempdir");
        let config_file = dir.path().join("config.toml");

        fs::write(
            &config_file,
            r#"
[watch]
directory="/downloads"

[notifications]
enabled=true
provider="ntfy"

[notifications.ntfy]
server="https://ntfy.example.com"
topic="xdcc-extractor"
token=""
"#,
        )
        .expect("write");

        let config = load_config(&config_file).expect("load config");
        assert!(config.notifications.enabled);
        assert!(config.notifications.ntfy.token.is_empty());
    }

    #[test]
    fn rejects_invalid_ntfy_topic() {
        let dir = tempdir().expect("tempdir");
        let config_file = dir.path().join("config.toml");

        fs::write(
            &config_file,
            r#"
[watch]
directory="/downloads"

[notifications]
enabled=true
provider="ntfy"

[notifications.ntfy]
server="https://ntfy.example.com"
topic="invalid topic"
"#,
        )
        .expect("write");

        let err = load_config(&config_file).expect_err("should fail");
        assert!(format!("{:?}", err).contains("topic"));
    }

    #[test]
    fn rejects_invalid_retry_config() {
        let dir = tempdir().expect("tempdir");
        let config_file = dir.path().join("config.toml");

        fs::write(
            &config_file,
            r#"
[watch]
directory="/downloads"

[retry]
base_delay=120
max_delay=60
"#,
        )
        .expect("write");

        let err = load_config(&config_file).expect_err("should fail");
        assert!(format!("{:?}", err).contains("retry.max_delay"));
    }
}

#[cfg(test)]
mod debug_redaction_tests {
    use super::*;

    #[test]
    fn ntfy_debug_output_redacts_token() {
        let ntfy = NtfyConfig {
            server: "https://ntfy.example.com".to_string(),
            topic: "xdcc-extractor".to_string(),
            token: "tk_super-secret-token".to_string(),
            priority_success: 3,
            priority_error: 5,
            notify_on_success: true,
            notify_on_error: true,
            notify_on_every_error: false,
            notify_after_attempts: 3,
        };

        let debug = format!("{:?}", ntfy);

        assert!(debug.contains("<redacted>"));
        assert!(!debug.contains("tk_super-secret-token"));
    }

    #[test]
    fn gotify_debug_output_redacts_token() {
        let gotify = GotifyConfig {
            enabled: true,
            url: "https://gotify.example.com".to_string(),
            token: "super-secret-token".to_string(),
            priority_success: 3,
            priority_error: 8,
            notify_on_success: true,
            notify_on_error: true,
            notify_on_every_error: false,
            notify_after_attempts: 3,
        };

        let debug = format!("{:?}", gotify);

        assert!(debug.contains("<redacted>"));
        assert!(!debug.contains("super-secret-token"));
    }
}

fn default_web_bind() -> String {
    "0.0.0.0:8099".to_string()
}
