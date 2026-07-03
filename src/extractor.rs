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
    pub release_root: PathBuf,
    pub archive: PathBuf,
    pub output_dir: PathBuf,
    pub cleanup_files: Vec<PathBuf>,
}

pub fn process_release(
    target: &Path,
    delete_archives: bool,
    dry_run: bool,
    keep_failed: bool,
) -> Result<()> {
    info!("Prüfe Release: {}", target.display());

    let plan = create_extract_plan(target)?;

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

pub fn create_extract_plan(target: &Path) -> Result<ExtractPlan> {
    if target.is_file() {
        return create_flat_extract_plan(target);
    }

    create_folder_extract_plan(target)
}

fn create_folder_extract_plan(release_dir: &Path) -> Result<ExtractPlan> {
    let archive = find_archive_start_in_dir(release_dir)?.with_context(|| {
        format!(
            "Kein unterstütztes Archiv gefunden in {}",
            release_dir.display()
        )
    })?;

    let output_dir = release_dir.join("_extracted");
    let cleanup_files = find_cleanup_files_in_dir(release_dir)?;

    Ok(ExtractPlan {
        release_root: release_dir.to_path_buf(),
        archive,
        output_dir,
        cleanup_files,
    })
}

fn create_flat_extract_plan(archive: &Path) -> Result<ExtractPlan> {
    let release_root = archive
        .parent()
        .with_context(|| format!("Archiv hat keinen Parent: {}", archive.display()))?
        .to_path_buf();

    let release_name = flat_release_name(archive)?;
    let output_dir = release_root.join("_extracted").join(release_name);

    let cleanup_files = find_related_flat_cleanup_files(archive)?;

    Ok(ExtractPlan {
        release_root,
        archive: archive.to_path_buf(),
        output_dir,
        cleanup_files,
    })
}

pub fn has_archive_start(target: &Path) -> Result<bool> {
    if target.is_file() {
        return Ok(is_archive_start_file(target));
    }

    Ok(find_archive_start_in_dir(target)?.is_some())
}

pub fn is_archive_related_file(path: &Path) -> bool {
    let Some(file_name) = path.file_name() else {
        return false;
    };

    let file_name = file_name.to_string_lossy();
    is_archive_file_name(&file_name)
}

pub fn root_archive_target(path: &Path) -> Option<PathBuf> {
    if !is_archive_related_file(path) {
        return None;
    }

    let file_name = path.file_name()?.to_string_lossy().to_string();
    let parent = path.parent()?;

    let part_re = Regex::new(r"(?i)^(?P<base>.+)\.part\d+\.rar$").ok()?;
    if let Some(caps) = part_re.captures(&file_name) {
        let base = caps.name("base")?.as_str();
        return Some(parent.join(format!("{}.part01.rar", base)));
    }

    let split_re = Regex::new(r"(?i)^(?P<base>.+)\.\d{3}$").ok()?;
    if let Some(caps) = split_re.captures(&file_name) {
        let base = caps.name("base")?.as_str();
        return Some(parent.join(format!("{}.001", base)));
    }

    let legacy_re = Regex::new(r"(?i)^(?P<base>.+)\.r\d{2}$").ok()?;
    if let Some(caps) = legacy_re.captures(&file_name) {
        let base = caps.name("base")?.as_str();
        return Some(parent.join(format!("{}.rar", base)));
    }

    Some(path.to_path_buf())
}

fn verify_archive(archive: &Path) -> Result<()> {
    if is_rar_archive_path(archive) {
        return verify_archive_with_unrar(archive);
    }

    verify_archive_with_7z(archive)
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

    if is_rar_archive_path(&plan.archive) {
        return extract_archive_with_unrar(plan);
    }

    extract_archive_with_7z(plan)
}

fn verify_archive_with_unrar(archive: &Path) -> Result<()> {
    info!("Starte Archivprüfung mit unrar: {}", archive.display());

    let output = Command::new("unrar")
        .arg("t")
        .arg("-idq")
        .arg(archive)
        .output()
        .with_context(|| "Konnte unrar nicht starten. Ist unrar im Container installiert?")?;

    check_command_success("Archivprüfung", "unrar", archive, output)
}

fn verify_archive_with_7z(archive: &Path) -> Result<()> {
    info!("Starte Archivprüfung mit 7z: {}", archive.display());

    let output = Command::new("7z")
        .arg("t")
        .arg(archive)
        .output()
        .with_context(|| "Konnte 7z nicht starten. Ist p7zip-full installiert?")?;

    check_command_success("Archivprüfung", "7z", archive, output)
}

fn extract_archive_with_unrar(plan: &ExtractPlan) -> Result<()> {
    info!("Starte Entpackung mit unrar: {}", plan.archive.display());

    let output = Command::new("unrar")
        .arg("x")
        .arg("-o+")
        .arg("-idq")
        .arg(&plan.archive)
        .arg(&plan.output_dir)
        .output()
        .with_context(|| "Konnte unrar nicht starten. Ist unrar im Container installiert?")?;

    check_command_success("Entpackung", "unrar", &plan.archive, output)
}

fn extract_archive_with_7z(plan: &ExtractPlan) -> Result<()> {
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

    check_command_success("Entpackung", "7z", &plan.archive, output)
}

fn check_command_success(
    action: &str,
    tool: &str,
    archive: &Path,
    output: std::process::Output,
) -> Result<()> {
    if output.status.success() {
        return Ok(());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    bail!(
        "{} fehlgeschlagen mit {}: {}\n\nSTDOUT:\n{}\n\nSTDERR:\n{}",
        action,
        tool,
        archive.display(),
        stdout,
        stderr
    );
}

fn is_rar_archive_path(path: &Path) -> bool {
    let Some(file_name) = path.file_name() else {
        return false;
    };

    let lower = file_name.to_string_lossy().to_lowercase();

    lower.ends_with(".rar")
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
            if !is_safe_cleanup_file(&plan.release_root, file) {
                bail!("Unsicherer Cleanup-Pfad blockiert: {}", file.display());
            }

            info!("Dry-Run Cleanup-Kandidat: {}", file.display());
        }

        warn!("Dry-Run aktiv: Es wurde nichts gelöscht.");
        return Ok(());
    }

    warn!("Cleanup aktiv: Archivdateien werden jetzt gelöscht.");

    for file in &plan.cleanup_files {
        if !is_safe_cleanup_file(&plan.release_root, file) {
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

fn is_safe_cleanup_file(release_root: &Path, file: &Path) -> bool {
    if file.parent() != Some(release_root) {
        return false;
    }

    let Some(file_name) = file.file_name() else {
        return false;
    };

    let file_name = file_name.to_string_lossy();

    if !is_archive_file_name(&file_name) {
        return false;
    }

    is_regular_file(file)
}

fn find_cleanup_files_in_dir(release_dir: &Path) -> Result<Vec<PathBuf>> {
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

fn find_related_flat_cleanup_files(archive: &Path) -> Result<Vec<PathBuf>> {
    let parent = archive
        .parent()
        .with_context(|| format!("Archiv hat keinen Parent: {}", archive.display()))?;

    let archive_name = archive
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .with_context(|| format!("Archiv hat keinen Dateinamen: {}", archive.display()))?;

    let group_prefix = cleanup_group_prefix(&archive_name);

    let mut files = Vec::new();

    for entry in fs::read_dir(parent)
        .with_context(|| format!("Kann Ordner nicht lesen: {}", parent.display()))?
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

        if !is_archive_file_name(&file_name) {
            continue;
        }

        if belongs_to_cleanup_group(&file_name, &group_prefix) {
            files.push(path);
        }
    }

    files.sort();

    Ok(files)
}

fn cleanup_group_prefix(file_name: &str) -> String {
    let part_re = Regex::new(r"(?i)^(?P<base>.+)\.part\d+\.rar$").expect("Invalid part regex");
    if let Some(caps) = part_re.captures(file_name) {
        return caps["base"].to_string();
    }

    let split_re = Regex::new(r"(?i)^(?P<base>.+)\.\d{3}$").expect("Invalid split regex");
    if let Some(caps) = split_re.captures(file_name) {
        return caps["base"].to_string();
    }

    let legacy_re = Regex::new(r"(?i)^(?P<base>.+)\.r\d{2}$").expect("Invalid legacy regex");
    if let Some(caps) = legacy_re.captures(file_name) {
        return caps["base"].to_string();
    }

    strip_archive_extension(file_name)
}

fn belongs_to_cleanup_group(file_name: &str, group_prefix: &str) -> bool {
    let lower = file_name.to_lowercase();
    let prefix = group_prefix.to_lowercase();

    lower == format!("{}.rar", prefix)
        || lower == format!("{}.zip", prefix)
        || lower == format!("{}.7z", prefix)
        || lower == format!("{}.tar", prefix)
        || lower.starts_with(&format!("{}.part", prefix)) && lower.ends_with(".rar")
        || lower.starts_with(&format!("{}.", prefix))
            && is_numbered_suffix(&lower[prefix.len() + 1..])
        || lower.starts_with(&format!("{}.r", prefix))
}

fn is_numbered_suffix(suffix: &str) -> bool {
    suffix.len() == 3 && suffix.chars().all(|c| c.is_ascii_digit())
}

fn find_archive_start_in_dir(release_dir: &Path) -> Result<Option<PathBuf>> {
    let mut candidates = Vec::new();

    for entry in fs::read_dir(release_dir)
        .with_context(|| format!("Kann Release-Ordner nicht lesen: {}", release_dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();

        if !is_regular_file(&path) {
            continue;
        }

        if is_archive_start_file(&path) {
            candidates.push(path);
        }
    }

    candidates.sort();

    Ok(candidates.first().cloned())
}

fn is_archive_start_file(path: &Path) -> bool {
    let Some(file_name) = path.file_name() else {
        return false;
    };

    let file_name = file_name.to_string_lossy();
    let lower = file_name.to_lowercase();

    let part01_re = Regex::new(r"(?i)\.part0*1\.rar$").expect("Invalid part01 regex");
    let rar_part_re = Regex::new(r"(?i)\.part\d+\.rar$").expect("Invalid rar part regex");
    let split001_re = Regex::new(r"(?i)\.001$").expect("Invalid split001 regex");

    part01_re.is_match(&file_name)
        || lower.ends_with(".rar") && !rar_part_re.is_match(&file_name)
        || lower.ends_with(".zip")
        || lower.ends_with(".7z")
        || lower.ends_with(".tar")
        || split001_re.is_match(&file_name)
}

fn flat_release_name(archive: &Path) -> Result<String> {
    let file_name = archive
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .with_context(|| format!("Archiv hat keinen Dateinamen: {}", archive.display()))?;

    Ok(sanitize_name(&cleanup_group_prefix(&file_name)))
}

fn strip_archive_extension(file_name: &str) -> String {
    let lower = file_name.to_lowercase();

    for suffix in [".part01.rar", ".rar", ".zip", ".7z", ".tar", ".001"] {
        if lower.ends_with(suffix) {
            let cut = file_name.len() - suffix.len();
            return file_name[..cut].to_string();
        }
    }

    file_name.to_string()
}

fn sanitize_name(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

fn is_regular_file(path: &Path) -> bool {
    match fs::symlink_metadata(path) {
        Ok(metadata) => metadata.file_type().is_file(),
        Err(_) => false,
    }
}

fn is_archive_file_name(file_name: &str) -> bool {
    let lower = file_name.to_lowercase();

    if lower.ends_with(".rar")
        || lower.ends_with(".zip")
        || lower.ends_with(".7z")
        || lower.ends_with(".tar")
    {
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
