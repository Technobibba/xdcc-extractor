use crate::config::{NotificationConfig, NtfyConfig};
use anyhow::{Context, Result, bail};
use serde_json::{Value, json};
use std::path::Path;
use tracing::{info, warn};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationEvent {
    WorkerStarted,
    ProcessingStarted,
    ProcessingSucceeded,
    ProcessingFailed,
    Test,
}

impl NotificationEvent {
    fn title(self) -> &'static str {
        match self {
            Self::WorkerStarted => "XDCC Extractor: Worker gestartet",
            Self::ProcessingStarted => "XDCC Extractor: Verarbeitung gestartet",
            Self::ProcessingSucceeded => "XDCC Extractor: Verarbeitung erfolgreich",
            Self::ProcessingFailed => "XDCC Extractor: Verarbeitung fehlgeschlagen",
            Self::Test => "XDCC Extractor: Testnachricht",
        }
    }

    fn tags(self) -> &'static [&'static str] {
        match self {
            Self::WorkerStarted => &["rocket", "gear"],
            Self::ProcessingStarted => &["package", "hourglass_flowing_sand"],
            Self::ProcessingSucceeded => &["white_check_mark", "package"],
            Self::ProcessingFailed => &["warning", "package"],
            Self::Test => &["white_check_mark", "test_tube"],
        }
    }
}

#[derive(Debug, Clone)]
pub struct Notifications {
    config: NotificationConfig,
}

impl Notifications {
    pub fn new(config: NotificationConfig) -> Self {
        Self { config }
    }

    pub fn enabled(&self) -> bool {
        self.config.enabled
    }

    pub fn provider(&self) -> &str {
        self.config.provider.trim()
    }

    pub fn send_worker_started(&self) {
        let ntfy = &self.config.ntfy;

        if !self.enabled() || !ntfy.notify_on_worker_start {
            return;
        }

        if let Err(err) = self.send_ntfy(
            NotificationEvent::WorkerStarted,
            "Der XDCC-Extractor-Worker wurde gestartet und überwacht die konfigurierten Verzeichnisse.",
            ntfy.priority_success,
        ) {
            warn!("ntfy-Worker-Startmeldung fehlgeschlagen: {err:#}");
        }
    }

    pub fn send_processing_started(&self, release: &Path) {
        let ntfy = &self.config.ntfy;

        if !self.enabled() || !ntfy.notify_on_processing_start {
            return;
        }

        let message = format!(
            "Verarbeitung gestartet:
{}",
            release.display()
        );

        if let Err(err) = self.send_ntfy(
            NotificationEvent::ProcessingStarted,
            &message,
            ntfy.priority_success,
        ) {
            warn!("ntfy-Startmeldung fehlgeschlagen: {err:#}");
        }
    }

    pub fn send_success(&self, release: &Path) {
        let ntfy = &self.config.ntfy;

        if !self.enabled() || !ntfy.notify_on_success {
            return;
        }

        let message = format!("Release erfolgreich verarbeitet:\n{}", release.display());

        if let Err(err) = self.send_ntfy(
            NotificationEvent::ProcessingSucceeded,
            &message,
            ntfy.priority_success,
        ) {
            warn!("ntfy-Erfolgsmeldung fehlgeschlagen: {err:#}");
        }
    }

    pub fn send_failure(&self, release: &Path, attempts: u64, error: &str) {
        let ntfy = &self.config.ntfy;

        if !self.enabled() || !ntfy.notify_on_error {
            return;
        }

        if !ntfy.notify_on_every_error {
            if attempts < ntfy.notify_after_attempts {
                info!(
                    "ntfy-Fehlermeldung übersprungen: Versuch {}/{}",
                    attempts, ntfy.notify_after_attempts
                );
                return;
            }

            if attempts > ntfy.notify_after_attempts {
                info!(
                    "ntfy-Fehlermeldung wurde bereits ab Versuch {} gesendet. Aktueller Versuch: {}",
                    ntfy.notify_after_attempts, attempts
                );
                return;
            }
        }

        let message = format!(
            "Release fehlgeschlagen:\n{}\n\nFehlversuche: {}\n\nFehler:\n{}",
            release.display(),
            attempts,
            shorten(error, 1200)
        );

        if let Err(err) = self.send_ntfy(
            NotificationEvent::ProcessingFailed,
            &message,
            ntfy.priority_error,
        ) {
            warn!("ntfy-Fehlermeldung fehlgeschlagen: {err:#}");
        }
    }

    pub fn send_test(&self) -> Result<()> {
        if !self.enabled() {
            bail!("Benachrichtigungen sind deaktiviert");
        }

        self.send_ntfy(
            NotificationEvent::Test,
            "Die ntfy-Konfiguration des XDCC Extractors funktioniert.",
            self.config.ntfy.priority_success,
        )
    }

    fn send_ntfy(&self, event: NotificationEvent, message: &str, priority: u8) -> Result<()> {
        if self.provider() != "ntfy" {
            bail!(
                "Nicht unterstützter Benachrichtigungsanbieter: {}",
                self.provider()
            );
        }

        let ntfy = &self.config.ntfy;
        validate_runtime_config(ntfy)?;

        let endpoint = ntfy.server.trim_end_matches('/');
        let payload = build_payload(ntfy, event, message, priority);
        let mut request = ureq::post(endpoint).set("Content-Type", "application/json");

        if !ntfy.token.trim().is_empty() {
            request = request.set("Authorization", &format!("Bearer {}", ntfy.token.trim()));
        }

        let response = request
            .send_json(payload)
            .with_context(|| format!("ntfy Request an {endpoint} fehlgeschlagen"))?;

        info!(
            "ntfy-Meldung '{}' gesendet: HTTP {}",
            event.title(),
            response.status()
        );

        Ok(())
    }
}

fn validate_runtime_config(ntfy: &NtfyConfig) -> Result<()> {
    if ntfy.server.trim().is_empty() {
        bail!("ntfy ist aktiviert, aber server ist leer");
    }

    if ntfy.topic.trim().is_empty() {
        bail!("ntfy ist aktiviert, aber topic ist leer");
    }

    Ok(())
}

fn build_payload(
    ntfy: &NtfyConfig,
    event: NotificationEvent,
    message: &str,
    priority: u8,
) -> Value {
    json!({
        "topic": ntfy.topic.trim(),
        "title": event.title(),
        "message": message,
        "priority": priority,
        "tags": event.tags(),
    })
}

fn shorten(input: &str, max_chars: usize) -> String {
    if input.chars().count() <= max_chars {
        return input.to_string();
    }

    let mut output: String = input.chars().take(max_chars).collect();
    output.push_str("\n... gekürzt ...");
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ntfy_config() -> NtfyConfig {
        NtfyConfig {
            server: "https://ntfy.example.org/".to_string(),
            topic: "xdcc-extractor".to_string(),
            token: "secret-token".to_string(),
            priority_success: 3,
            priority_error: 5,
            notify_on_worker_start: false,
            notify_on_processing_start: false,
            notify_on_success: true,
            notify_on_error: true,
            notify_on_every_error: false,
            notify_after_attempts: 3,
        }
    }

    #[test]
    fn worker_started_payload_uses_worker_metadata() {
        let payload = build_payload(
            &ntfy_config(),
            NotificationEvent::WorkerStarted,
            "gestartet",
            3,
        );

        assert_eq!(payload["title"], "XDCC Extractor: Worker gestartet");
        assert_eq!(payload["tags"][0], "rocket");
    }

    #[test]
    fn processing_started_payload_uses_processing_metadata() {
        let payload = build_payload(
            &ntfy_config(),
            NotificationEvent::ProcessingStarted,
            "gestartet",
            3,
        );

        assert_eq!(payload["title"], "XDCC Extractor: Verarbeitung gestartet");
        assert_eq!(payload["tags"][0], "package");
    }

    #[test]
    fn success_payload_contains_topic_title_priority_and_tags() {
        let payload = build_payload(
            &ntfy_config(),
            NotificationEvent::ProcessingSucceeded,
            "fertig",
            3,
        );

        assert_eq!(payload["topic"], "xdcc-extractor");
        assert_eq!(payload["title"], "XDCC Extractor: Verarbeitung erfolgreich");
        assert_eq!(payload["message"], "fertig");
        assert_eq!(payload["priority"], 3);
        assert_eq!(payload["tags"][0], "white_check_mark");
    }

    #[test]
    fn failure_payload_uses_error_metadata() {
        let payload = build_payload(
            &ntfy_config(),
            NotificationEvent::ProcessingFailed,
            "fehlgeschlagen",
            5,
        );

        assert_eq!(
            payload["title"],
            "XDCC Extractor: Verarbeitung fehlgeschlagen"
        );
        assert_eq!(payload["priority"], 5);
        assert_eq!(payload["tags"][0], "warning");
    }

    #[test]
    fn shorten_preserves_short_messages() {
        assert_eq!(shorten("kurz", 10), "kurz");
    }

    #[test]
    fn shorten_limits_unicode_by_characters() {
        assert_eq!(shorten("äöüß", 3), "äöü\n... gekürzt ...");
    }

    #[test]
    fn runtime_validation_accepts_public_topic_without_token() {
        let mut config = ntfy_config();
        config.token.clear();
        assert!(validate_runtime_config(&config).is_ok());
    }

    #[test]
    fn runtime_validation_rejects_missing_topic() {
        let mut config = ntfy_config();
        config.topic.clear();
        assert!(validate_runtime_config(&config).is_err());
    }
}
