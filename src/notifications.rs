use crate::config::NotificationConfig;
use anyhow::{Context, Result, bail};
use serde_json::json;
use std::path::Path;
use tracing::{info, warn};

#[derive(Debug, Clone)]
pub struct Notifications {
    config: NotificationConfig,
}

impl Notifications {
    pub fn new(config: NotificationConfig) -> Self {
        Self { config }
    }

    pub fn gotify_enabled(&self) -> bool {
        self.config.gotify.enabled
    }

    pub fn send_success(&self, release: &Path) {
        let gotify = &self.config.gotify;

        if !gotify.enabled || !gotify.notify_on_success {
            return;
        }

        let title = "XDCC Extractor: Erfolg";
        let message = format!("Release erfolgreich verarbeitet:\n{}", release.display());

        if let Err(err) = self.send_gotify(title, &message, gotify.priority_success) {
            warn!("Gotify-Erfolgsmeldung fehlgeschlagen: {:?}", err);
        }
    }

    pub fn send_failure(&self, release: &Path, attempts: u64, error: &str) {
        let gotify = &self.config.gotify;

        if !gotify.enabled || !gotify.notify_on_error {
            return;
        }

        if !gotify.notify_on_every_error {
            if attempts < gotify.notify_after_attempts {
                info!(
                    "Gotify-Fehlermeldung übersprungen: Versuch {}/{}",
                    attempts, gotify.notify_after_attempts
                );
                return;
            }

            if attempts > gotify.notify_after_attempts {
                info!(
                    "Gotify-Fehlermeldung wurde bereits ab Versuch {} gesendet. Aktueller Versuch: {}",
                    gotify.notify_after_attempts, attempts
                );
                return;
            }
        }

        let title = "XDCC Extractor: Fehler";
        let message = format!(
            "Release fehlgeschlagen:\n{}\n\nFehlversuche: {}\n\nFehler:\n{}",
            release.display(),
            attempts,
            shorten(error, 1200)
        );

        if let Err(err) = self.send_gotify(title, &message, gotify.priority_error) {
            warn!("Gotify-Fehlermeldung fehlgeschlagen: {:?}", err);
        }
    }

    fn send_gotify(&self, title: &str, message: &str, priority: i32) -> Result<()> {
        let gotify = &self.config.gotify;

        if !gotify.enabled {
            return Ok(());
        }

        if gotify.url.trim().is_empty() {
            bail!("Gotify ist aktiviert, aber url ist leer");
        }

        if gotify.token.trim().is_empty() {
            bail!("Gotify ist aktiviert, aber token ist leer");
        }

        let endpoint = format!(
            "{}/message?token={}",
            gotify.url.trim_end_matches('/'),
            gotify.token
        );

        let payload = json!({
            "title": title,
            "message": message,
            "priority": priority,
        });

        let response = ureq::post(&endpoint)
            .set("Content-Type", "application/json")
            .send_json(payload)
            .with_context(|| "Gotify Request fehlgeschlagen")?;

        info!("Gotify-Meldung gesendet: HTTP {}", response.status());

        Ok(())
    }
}

fn shorten(input: &str, max_chars: usize) -> String {
    if input.chars().count() <= max_chars {
        return input.to_string();
    }

    let mut output: String = input.chars().take(max_chars).collect();
    output.push_str("\n... gekürzt ...");
    output
}
