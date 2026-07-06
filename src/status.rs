use anyhow::{Context, Result};
use std::{env, fs, path::Path};

pub fn is_version_command() -> bool {
    env::args().any(|arg| arg == "--version" || arg == "-V" || arg == "version")
}

pub fn print_version() {
    println!("xdcc-extractor {}", env!("CARGO_PKG_VERSION"));
}

pub fn is_status_command() -> bool {
    env::args().any(|arg| arg == "--status" || arg == "status")
}

pub fn run_from_args() -> Result<()> {
    let config_path = config_path_from_args();
    print_status(&config_path)
}

fn config_path_from_args() -> String {
    let mut args = env::args().skip(1);
    let mut config_path = env::var("XDCC_CONFIG").unwrap_or_else(|_| "config.toml".to_string());

    while let Some(arg) = args.next() {
        if (arg == "--config" || arg == "-c") && args.next().is_some() {
            config_path = env::args()
                .skip_while(|a| a != "--config" && a != "-c")
                .nth(1)
                .unwrap_or(config_path);
        }
    }

    config_path
}

pub fn print_status(config_path: &str) -> Result<()> {
    println!("== XDCC Extractor Status ==");
    println!();

    println!("Version:");
    println!("  {}", env!("CARGO_PKG_VERSION"));
    println!();

    println!("Binary:");
    println!("  OK");
    println!();

    println!("Config:");
    println!("  Pfad: {}", config_path);

    let config_content = fs::read_to_string(config_path)
        .with_context(|| format!("Konnte Config nicht lesen: {}", config_path))?;

    println!("  Status: OK");
    println!();

    let config: toml::Value = toml::from_str(&config_content)
        .with_context(|| format!("Konnte Config nicht parsen: {}", config_path))?;

    print_path_status(
        "Watch directory",
        toml_string(&config, "watch", "directory"),
    );
    print_path_status(
        "Output directory",
        toml_string(&config, "output", "directory"),
    );
    print_path_status(
        "History directory",
        toml_string(&config, "history", "directory"),
    );

    println!();

    if let Some(password_file) = toml_string(&config, "extract", "password_file") {
        if password_file.trim().is_empty() {
            println!("Password file:");
            println!("  nicht konfiguriert");
        } else {
            println!("Password file:");
            println!("  Pfad: {}", password_file);

            if Path::new(password_file).is_file() {
                let count = count_passwords(password_file)?;
                println!("  Status: OK");
                println!("  Einträge: {}", count);
            } else {
                println!("  Status: FEHLT");
            }
        }

        println!();
    }

    if let Some(history_dir) = toml_string(&config, "history", "directory") {
        print_history_summary(history_dir)?;
        println!();
    }

    println!("Gotify:");
    let gotify_enabled =
        toml_bool_nested(&config, "notifications", "gotify", "enabled").unwrap_or(false);
    println!("  enabled: {}", gotify_enabled);

    if let Some(url) = toml_string_nested(&config, "notifications", "gotify", "url") {
        println!(
            "  url: {}",
            if url.trim().is_empty() { "<leer>" } else { url }
        );
    }

    let token_set = toml_string_nested(&config, "notifications", "gotify", "token")
        .map(|token| !token.trim().is_empty())
        .unwrap_or(false);

    println!("  token gesetzt: {}", token_set);
    println!();

    println!("Status-Prüfung abgeschlossen.");

    Ok(())
}

fn print_path_status(label: &str, value: Option<&str>) {
    println!("{}:", label);

    match value {
        Some(path) if !path.trim().is_empty() => {
            println!("  Pfad: {}", path);

            if Path::new(path).exists() {
                println!("  Status: OK");
            } else {
                println!("  Status: FEHLT");
            }
        }
        _ => {
            println!("  Status: nicht konfiguriert");
        }
    }

    println!();
}

fn print_history_summary(history_dir: &str) -> Result<()> {
    println!("History:");

    let path = Path::new(history_dir);

    if !path.is_dir() {
        println!("  Status: FEHLT");
        return Ok(());
    }

    let mut done = 0;
    let mut failed = 0;
    let mut failed_entries = Vec::new();

    for entry in fs::read_dir(path)
        .with_context(|| format!("Konnte History-Ordner nicht lesen: {}", history_dir))?
    {
        let entry = entry?;
        let entry_path = entry.path();
        let file_name = entry.file_name().to_string_lossy().to_string();

        if file_name.ends_with(".done") {
            done += 1;
        }

        if file_name.ends_with(".failed") {
            failed += 1;

            let modified = entry
                .metadata()
                .and_then(|metadata| metadata.modified())
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH);

            failed_entries.push((modified, file_name, entry_path));
        }
    }

    failed_entries.sort_by(|a, b| b.0.cmp(&a.0));

    println!("  Status: OK");
    println!("  done: {}", done);
    println!("  failed: {}", failed);

    if !failed_entries.is_empty() {
        println!("  letzte Fehler:");

        for (_, file_name, entry_path) in failed_entries.iter().take(5) {
            println!("    - {}", file_name);

            if let Ok(content) = fs::read_to_string(entry_path) {
                if let Some(reason) = first_error_line(&content) {
                    println!("      {}", reason);
                }
            }
        }
    }

    Ok(())
}

fn count_passwords(path: &str) -> Result<usize> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Konnte Passwortdatei nicht lesen: {}", path))?;

    Ok(content
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .filter(|line| !line.starts_with('#'))
        .count())
}

fn toml_string<'a>(config: &'a toml::Value, section: &str, key: &str) -> Option<&'a str> {
    config.get(section)?.get(key)?.as_str()
}

fn toml_string_nested<'a>(
    config: &'a toml::Value,
    section: &str,
    subsection: &str,
    key: &str,
) -> Option<&'a str> {
    config.get(section)?.get(subsection)?.get(key)?.as_str()
}

fn toml_bool_nested(
    config: &toml::Value,
    section: &str,
    subsection: &str,
    key: &str,
) -> Option<bool> {
    config.get(section)?.get(subsection)?.get(key)?.as_bool()
}

fn first_error_line(content: &str) -> Option<String> {
    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with("Fehlerklasse:") || trimmed.starts_with("Grund:") {
            return Some(trimmed.to_string());
        }
    }

    content
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(|line| {
            let mut value = line.to_string();

            if value.chars().count() > 120 {
                value = value.chars().take(120).collect();
                value.push_str("...");
            }

            value
        })
}
