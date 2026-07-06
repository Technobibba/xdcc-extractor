use crate::{config::Config, history::History};
use anyhow::{Context, Result, bail};
use std::{env, path::Path};

pub fn is_clear_failed_command() -> bool {
    env::args().any(|arg| arg == "--clear-failed" || arg == "clear-failed")
}

pub fn run_clear_failed_from_args() -> Result<()> {
    let release_path = clear_failed_target_from_args()
        .context("Kein Release-Pfad angegeben. Beispiel: --clear-failed /downloads/Release.zip")?;

    let config_path = crate::status::config_path_from_args();
    let config = Config::load(&config_path)?;

    let history = History::new(Path::new(&config.history.directory))?;

    let release = Path::new(&release_path);

    println!("== XDCC Extractor Clear Failed ==");
    println!();
    println!("Config:");
    println!("  {}", config_path);
    println!();
    println!("History:");
    println!("  {}", config.history.directory);
    println!();
    println!("Release:");
    println!("  {}", release.display());
    println!();

    let removed = history.clear_failed(release)?;

    if removed {
        println!("Failed-Marker wurde gelöscht.");
        println!("Das Release kann beim nächsten Worker-Lauf erneut verarbeitet werden.");
    } else {
        println!("Kein Failed-Marker vorhanden.");
        println!("Es wurde nichts geändert.");
    }

    Ok(())
}

fn clear_failed_target_from_args() -> Option<String> {
    let mut args = env::args().skip(1);

    while let Some(arg) = args.next() {
        if arg == "--clear-failed" || arg == "clear-failed" {
            return args.next();
        }
    }

    None
}

pub fn validate_clear_failed_args() -> Result<()> {
    if !is_clear_failed_command() {
        return Ok(());
    }

    let Some(target) = clear_failed_target_from_args() else {
        bail!("Option --clear-failed benötigt einen Release-Pfad");
    };

    if target.starts_with('-') {
        bail!("Option --clear-failed benötigt einen gültigen Release-Pfad");
    }

    Ok(())
}
