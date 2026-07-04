use anyhow::{Context, Result};
use std::{fs, path::Path};
use tracing::{info, warn};

pub fn load_passwords(path: &str) -> Result<Vec<String>> {
    if path.trim().is_empty() {
        info!("Keine Passwortdatei konfiguriert.");
        return Ok(Vec::new());
    }

    let password_path = Path::new(path);

    if !password_path.exists() {
        warn!(
            "Passwortdatei existiert nicht, fahre ohne Passwörter fort: {}",
            password_path.display()
        );
        return Ok(Vec::new());
    }

    let content = fs::read_to_string(password_path).with_context(|| {
        format!(
            "Konnte Passwortdatei nicht lesen: {}",
            password_path.display()
        )
    })?;

    let passwords = content
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .filter(|line| !line.starts_with('#'))
        .map(ToString::to_string)
        .collect::<Vec<_>>();

    info!(
        "Passwortdatei geladen: {} Passwort/Passwörter",
        passwords.len()
    );

    Ok(passwords)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn loads_passwords_and_ignores_comments() {
        let dir = tempdir().expect("tempdir");
        let file = dir.path().join("passwords.txt");

        fs::write(&file, "\n# Kommentar\nsecret1\n\nsecret2\n   secret3   \n").expect("write");

        let passwords = load_passwords(file.to_str().unwrap()).expect("load");

        assert_eq!(passwords, vec!["secret1", "secret2", "secret3"]);
    }

    #[test]
    fn empty_path_returns_empty_list() {
        let passwords = load_passwords("").expect("load");
        assert!(passwords.is_empty());
    }

    #[test]
    fn missing_file_returns_empty_list() {
        let passwords = load_passwords("/tmp/does-not-exist-xdcc-passwords.txt").expect("load");
        assert!(passwords.is_empty());
    }
}
