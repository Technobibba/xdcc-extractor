use anyhow::{Context, Result, bail};
use regex::Regex;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};
use tracing::info;

#[derive(Debug, Clone)]
pub struct ExtractPlan {
    pub archive: PathBuf,
}

pub fn process_release(release_dir: &Path, delete_archives: bool, keep_failed: bool) -> Result<()> {
    info!("Prüfe Release: {}", release_dir.display());

    let plan = create_extract_plan(release_dir)?;

    info!("Archiv-Start gefunden: {}", plan.archive.display());

    verify_archive(&plan.archive)?;

    info!("Archivprüfung erfolgreich: {}", plan.archive.display());
    info!("Es wird noch nichts entpackt.");
    info!("Konfiguration delete_archives={}", delete_archives);
    info!("Konfiguration keep_failed={}", keep_failed);

    Ok(())
}

pub fn create_extract_plan(release_dir: &Path) -> Result<ExtractPlan> {
    let archive = find_archive_start(release_dir)?.with_context(|| {
        format!(
            "Kein unterstütztes Archiv gefunden in {}",
            release_dir.display()
        )
    })?;

    Ok(ExtractPlan { archive })
}

fn verify_archive(archive: &Path) -> Result<()> {
    info!("Starte Archivprüfung mit 7z: {}", archive.display());

    let output = Command::new("7z")
        .arg("t")
        .arg(archive)
        .output()
        .with_context(|| "Konnte 7z nicht starten. Ist p7zip-full installiert?")?;

    if output.status.success() {
        return Ok(());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    bail!(
        "Archivprüfung fehlgeschlagen: {}\n\nSTDOUT:\n{}\n\nSTDERR:\n{}",
        archive.display(),
        stdout,
        stderr
    );
}

fn find_archive_start(release_dir: &Path) -> Result<Option<PathBuf>> {
    let part01_re = Regex::new(r"(?i)\.part0*1\.rar$")?;
    let rar_part_re = Regex::new(r"(?i)\.part\d+\.rar$")?;

    let mut part01_archives = Vec::new();
    let mut rar_archives = Vec::new();
    let mut zip_archives = Vec::new();
    let mut seven_zip_archives = Vec::new();

    for entry in fs::read_dir(release_dir)
        .with_context(|| format!("Kann Release-Ordner nicht lesen: {}", release_dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();

        if !path.is_file() {
            continue;
        }

        let Some(file_name) = path.file_name() else {
            continue;
        };

        let file_name = file_name.to_string_lossy();
        let lower = file_name.to_lowercase();

        if part01_re.is_match(&file_name) {
            part01_archives.push(path);
        } else if lower.ends_with(".rar") && !rar_part_re.is_match(&file_name) {
            rar_archives.push(path);
        } else if lower.ends_with(".zip") {
            zip_archives.push(path);
        } else if lower.ends_with(".7z") {
            seven_zip_archives.push(path);
        }
    }

    part01_archives.sort();
    rar_archives.sort();
    zip_archives.sort();
    seven_zip_archives.sort();

    if let Some(path) = part01_archives.first() {
        return Ok(Some(path.clone()));
    }

    if let Some(path) = rar_archives.first() {
        return Ok(Some(path.clone()));
    }

    if let Some(path) = zip_archives.first() {
        return Ok(Some(path.clone()));
    }

    if let Some(path) = seven_zip_archives.first() {
        return Ok(Some(path.clone()));
    }

    Ok(None)
}
