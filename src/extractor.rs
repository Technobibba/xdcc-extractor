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
    pub release_dir: PathBuf,
    pub archive: PathBuf,
    pub output_dir: PathBuf,
    pub cleanup_files: Vec<PathBuf>,
}

pub fn process_release(
    release_dir: &Path,
    delete_archives: bool,
    dry_run: bool,
    keep_failed: bool,
) -> Result<()> {
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

    execute_cleanup(&plan, delete_archives, dry_run)?;

    info!("Konfiguration delete_archives={}", delete_archives);
    info!("Konfiguration dry_run={}", dry_run);
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
    let cleanup_files = find_cleanup_files(release_dir)?;

    Ok(ExtractPlan {
        release_dir: release_dir.to_path_buf(),
        archive,
        output_dir,
        cleanup_files,
    })
}

pub fn has_archive_start(release_dir: &Path) -> Result<bool> {
    Ok(find_archive_start(release_dir)?.is_some())
}

pub fn is_archive_related_file(path: &Path) -> bool {
    let Some(file_name) = path.file_name() else {
        return false;
    };

    let file_name = file_name.to_string_lossy();
    is_archive_file_name(&file_name)
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

fn execute_cleanup(plan: &ExtractPlan, delete_archives: bool, dry_run: bool) -> Result<()> {
    if plan.cleanup_files.is_empty() {
        warn!("Cleanup: Keine Archivdateien gefunden.");
        return Ok(());
    }

    if !delete_archives {
        info!("Cleanup deaktiviert. Erkannte Archivdateien:");

        for file in &plan.cleanup_files {
            info!("Cleanup-Kandidat: {}", file.display());
        }

        warn!("Es wurde nichts gelöscht, weil delete_archives=false ist.");
        return Ok(());
    }

    if dry_run {
        warn!("Cleanup Dry-Run aktiv: Diese Archivdateien würden gelöscht werden:");

        for file in &plan.cleanup_files {
            if !is_safe_cleanup_file(&plan.release_dir, file) {
                bail!("Unsicherer Cleanup-Pfad blockiert: {}", file.display());
            }

            info!("Dry-Run Cleanup-Kandidat: {}", file.display());
        }

        warn!("Dry-Run aktiv: Es wurde nichts gelöscht.");
        return Ok(());
    }

    warn!("Cleanup aktiv: Archivdateien werden jetzt gelöscht.");

    for file in &plan.cleanup_files {
        if !is_safe_cleanup_file(&plan.release_dir, file) {
            bail!("Unsicherer Cleanup-Pfad blockiert: {}", file.display());
        }

        info!("Lösche Archivdatei: {}", file.display());

        fs::remove_file(file)
            .with_context(|| format!("Konnte Archivdatei nicht löschen: {}", file.display()))?;
    }

    info!(
        "Cleanup abgeschlossen: {} Datei(en) gelöscht",
        plan.cleanup_files.len()
    );

    Ok(())
}

fn is_safe_cleanup_file(release_dir: &Path, file: &Path) -> bool {
    if file.parent() != Some(release_dir) {
        return false;
    }

    let Some(file_name) = file.file_name() else {
        return false;
    };

    let file_name = file_name.to_string_lossy();

    if !is_archive_file_name(&file_name) {
        return false;
    }

    match fs::symlink_metadata(file) {
        Ok(metadata) => metadata.file_type().is_file(),
        Err(_) => false,
    }
}

fn find_cleanup_files(release_dir: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();

    for entry in fs::read_dir(release_dir)
        .with_context(|| format!("Kann Release-Ordner nicht lesen: {}", release_dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();

        if !is_regular_file(&path) {
            continue;
        }

        let Some(file_name) = path.file_name() else {
            continue;
        };

        let file_name = file_name.to_string_lossy();

        if is_archive_file_name(&file_name) {
            files.push(path);
        }
    }

    files.sort();

    Ok(files)
}

fn find_archive_start(release_dir: &Path) -> Result<Option<PathBuf>> {
    let part01_re = Regex::new(r"(?i)\.part0*1\.rar$")?;
    let rar_part_re = Regex::new(r"(?i)\.part\d+\.rar$")?;
    let split001_re = Regex::new(r"(?i)\.001$")?;

    let mut part01_archives = Vec::new();
    let mut rar_archives = Vec::new();
    let mut zip_archives = Vec::new();
    let mut seven_zip_archives = Vec::new();
    let mut split001_archives = Vec::new();

    for entry in fs::read_dir(release_dir)
        .with_context(|| format!("Kann Release-Ordner nicht lesen: {}", release_dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();

        if !is_regular_file(&path) {
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
        } else if split001_re.is_match(&file_name) {
            split001_archives.push(path);
        }
    }

    part01_archives.sort();
    rar_archives.sort();
    zip_archives.sort();
    seven_zip_archives.sort();
    split001_archives.sort();

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

    if let Some(path) = split001_archives.first() {
        return Ok(Some(path.clone()));
    }

    Ok(None)
}

fn is_regular_file(path: &Path) -> bool {
    match fs::symlink_metadata(path) {
        Ok(metadata) => metadata.file_type().is_file(),
        Err(_) => false,
    }
}

fn is_archive_file_name(file_name: &str) -> bool {
    let lower = file_name.to_lowercase();

    if lower.ends_with(".rar") || lower.ends_with(".zip") || lower.ends_with(".7z") {
        return true;
    }

    let legacy_rar_re = Regex::new(r"(?i)\.r\d{2}$").expect("Invalid legacy rar regex");
    if legacy_rar_re.is_match(file_name) {
        return true;
    }

    let split_re = Regex::new(r"(?i)\.\d{3}$").expect("Invalid split archive regex");
    if split_re.is_match(file_name) {
        return true;
    }

    false
}
