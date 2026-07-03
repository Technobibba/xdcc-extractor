use anyhow::{Context, Result, bail};
use regex::Regex;
use std::{
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    process::Command,
};
use tracing::{info, warn};
use walkdir::WalkDir;

#[derive(Debug, Clone)]
pub struct ExtractPlan {
    pub archive: PathBuf,
    pub output_dir: PathBuf,
}

pub fn process_release(release_dir: &Path, delete_archives: bool, keep_failed: bool) -> Result<()> {
    info!("Prüfe Release: {}", release_dir.display());

    let plan = create_extract_plan(release_dir)?;

    info!("Archiv-Start gefunden: {}", plan.archive.display());
    info!("Zielordner für Entpackung: {}", plan.output_dir.display());

    verify_archive(&plan.archive)?;
    info!("Archivprüfung erfolgreich: {}", plan.archive.display());

    extract_archive(&plan)?;
    info!("Entpackung abgeschlossen: {}", plan.output_dir.display());

    validate_extraction(&plan.output_dir)?;
    info!("Entpackung validiert: {}", plan.output_dir.display());

    info!("Archive werden in dieser Version noch nicht gelöscht.");
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

    let output_dir = release_dir.join("_extracted");

    Ok(ExtractPlan {
        archive,
        output_dir,
    })
}

pub fn has_archive_start(release_dir: &Path) -> Result<bool> {
    Ok(find_archive_start(release_dir)?.is_some())
}

pub fn is_archive_related_file(path: &Path) -> bool {
    let Some(file_name) = path.file_name() else {
        return false;
    };

    let lower = file_name.to_string_lossy().to_lowercase();

    lower.ends_with(".rar")
        || lower.ends_with(".zip")
        || lower.ends_with(".7z")
        || lower.ends_with(".001")
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

fn extract_archive(plan: &ExtractPlan) -> Result<()> {
    if plan.output_dir.exists() {
        warn!(
            "Zielordner existiert bereits und wird gelöscht: {}",
            plan.output_dir.display()
        );

        fs::remove_dir_all(&plan.output_dir).with_context(|| {
            format!(
                "Konnte bestehenden Zielordner nicht löschen: {}",
                plan.output_dir.display()
            )
        })?;
    }

    fs::create_dir_all(&plan.output_dir).with_context(|| {
        format!(
            "Konnte Zielordner nicht erstellen: {}",
            plan.output_dir.display()
        )
    })?;

    info!("Starte Entpackung mit 7z: {}", plan.archive.display());

    let mut output_arg = OsString::from("-o");
    output_arg.push(plan.output_dir.as_os_str());

    let output = Command::new("7z")
        .arg("x")
        .arg(&plan.archive)
        .arg(output_arg)
        .arg("-y")
        .output()
        .with_context(|| "Konnte 7z nicht starten. Ist p7zip-full installiert?")?;

    if output.status.success() {
        return Ok(());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    bail!(
        "Entpackung fehlgeschlagen: {}\n\nSTDOUT:\n{}\n\nSTDERR:\n{}",
        plan.archive.display(),
        stdout,
        stderr
    );
}

fn validate_extraction(output_dir: &Path) -> Result<()> {
    if !output_dir.exists() {
        bail!("Zielordner existiert nach Entpackung nicht");
    }

    let has_files = WalkDir::new(output_dir)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .any(|entry| entry.path().is_file());

    if !has_files {
        bail!("Entpackung scheint leer zu sein: {}", output_dir.display());
    }

    Ok(())
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
