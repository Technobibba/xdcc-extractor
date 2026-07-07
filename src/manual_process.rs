use crate::{config::Config, extractor, history::History, notifications::Notifications, passwords};
use anyhow::{Context, Result, bail};
use std::{env, path::Path};

pub fn is_process_command() -> bool {
    env::args().any(|arg| arg == "--process" || arg == "process")
}

pub fn run_from_args() -> Result<()> {
    let release_path = process_target_from_args()
        .context("Kein Release-Pfad angegeben. Beispiel: --process /downloads/Release.rar")?;

    let config_path = crate::status::config_path_from_args();
    let config = Config::load(&config_path)?;

    run_process(&config_path, &config, Path::new(&release_path))
}

fn process_target_from_args() -> Option<String> {
    let mut args = env::args().skip(1);

    while let Some(arg) = args.next() {
        if arg == "--process" || arg == "process" {
            return args.next();
        }
    }

    None
}

pub fn validate_process_args() -> Result<()> {
    if !is_process_command() {
        return Ok(());
    }

    let Some(target) = process_target_from_args() else {
        bail!("Option --process benötigt einen Release-Pfad");
    };

    if target.starts_with('-') {
        bail!("Option --process benötigt einen gültigen Release-Pfad");
    }

    Ok(())
}

pub fn run_process(config_path: &str, config: &Config, release: &Path) -> Result<()> {
    println!("== XDCC Extractor Manual Process ==");
    println!();
    println!("Config:");
    println!("  {}", config_path);
    println!();
    println!("Release:");
    println!("  {}", release.display());
    println!();

    if !release.exists() {
        bail!("Release-Pfad existiert nicht: {}", release.display());
    }

    if !extractor::has_archive_start(release)? {
        bail!(
            "Kein verarbeitbares Startarchiv gefunden: {}",
            release.display()
        );
    }

    let history = History::new(&config.history.directory)?;
    let notifications = Notifications::new(config.notifications.clone());

    if history.is_done(release) {
        println!("Status:");
        println!("  already done");
        println!();
        println!("Release wurde bereits erfolgreich verarbeitet. Es wurde nichts geändert.");
        return Ok(());
    }

    let failed_attempts = history.failed_attempts(release)?;

    if failed_attempts > 0 {
        bail!(
            "Release hat bereits einen Failed-Marker mit {} Fehlversuch(en).\nErst zurücksetzen mit:\nxdcc-extractor --clear-failed {}",
            failed_attempts,
            release.display()
        );
    }

    let passwords = passwords::load_passwords(&config.extract.password_file)?;

    println!("Optionen:");
    println!("  output: {}", config.output.directory);
    println!("  delete_archives: {}", config.extract.delete_archives);
    println!("  dry_run: {}", config.extract.dry_run);
    println!("  keep_failed: {}", config.extract.keep_failed);
    println!("  passwords: {}", passwords.len());
    println!();

    let result = extractor::process_release(
        release,
        Path::new(&config.output.directory),
        config.extract.delete_archives,
        config.extract.dry_run,
        config.extract.keep_failed,
        &passwords,
    );

    match result {
        Ok(()) => {
            history.mark_done(release)?;
            notifications.send_success(release);

            println!("Status:");
            println!("  success");
            println!();
            println!("Release erfolgreich verarbeitet.");
            Ok(())
        }
        Err(err) => {
            let error_text = format!("{:?}", err);

            history.mark_failed(release, &error_text)?;
            let attempts = history.failed_attempts(release).unwrap_or(1);

            notifications.send_failure(release, attempts, &error_text);

            bail!("Release-Verarbeitung fehlgeschlagen:\n{}", error_text)
        }
    }
}
