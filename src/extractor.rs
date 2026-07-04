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
    output_base: &Path,
    delete_archives: bool,
    dry_run: bool,
    keep_failed: bool,
) -> Result<()> {
    info!("Prüfe Release: {}", target.display());

    let plan = create_extract_plan(target, output_base)?;

    info!("Archiv-Start gefunden: {}", plan.archive.display());
    info!("Zielordner für Entpackung: {}", plan.output_dir.display());

    verify_archive(&plan.archive)?;
    info!("Archivprüfung erfolgreich: {}", plan.archive.display());

    extract_archive(&plan)?;
    info!("Entpackung abgeschlossen: {}", plan.output_dir.display());

    validate_extraction(&plan.output_dir)?;
    info!("Entpackung validiert: {}", plan.output_dir.display());

    execute_cleanup(&plan, delete_archives, dry_run)?;

    info!("Konfiguration output_base={}", output_base.display());
    info!("Konfiguration delete_archives={}", delete_archives);
    info!("Konfiguration dry_run={}", dry_run);
    info!("Konfiguration keep_failed={}", keep_failed);

    Ok(())
}

pub fn create_extract_plan(target: &Path, output_base: &Path) -> Result<ExtractPlan> {
    if target.is_file() {
        return create_flat_extract_plan(target, output_base);
    }

    create_folder_extract_plan(target, output_base)
}

fn create_folder_extract_plan(release_dir: &Path, output_base: &Path) -> Result<ExtractPlan> {
    let archive = find_archive_start_in_dir(release_dir)?.with_context(|| {
        format!(
            "Kein unterstütztes Archiv gefunden in {}",
            release_dir.display()
        )
    })?;

    let release_name = release_dir
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    let output_dir = output_base.join(sanitize_name(&release_name));
    let cleanup_files = find_cleanup_files_in_dir(release_dir)?;

    Ok(ExtractPlan {
        release_root: release_dir.to_path_buf(),
        archive,
        output_dir,
        cleanup_files,
    })
}

fn create_flat_extract_plan(archive: &Path, output_base: &Path) -> Result<ExtractPlan> {
    let release_root = archive
        .parent()
        .with_context(|| format!("Archiv hat keinen Parent: {}", archive.display()))?
        .to_path_buf();

    let release_name = flat_release_name(archive)?;
    let output_dir = output_base.join(release_name);

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

    if is_tar_archive_path(archive) {
        return verify_archive_with_tar(archive);
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

    if is_tar_archive_path(&plan.archive) {
        return extract_archive_with_tar(plan);
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

fn verify_archive_with_tar(archive: &Path) -> Result<()> {
    info!("Starte Archivprüfung mit tar: {}", archive.display());

    let output = Command::new("tar")
        .arg(tar_test_flag(archive)?)
        .arg(archive)
        .output()
        .with_context(|| "Konnte tar nicht starten. Ist tar im Container installiert?")?;

    check_command_success("Archivprüfung", "tar", archive, output)
}

fn extract_archive_with_tar(plan: &ExtractPlan) -> Result<()> {
    info!("Starte Entpackung mit tar: {}", plan.archive.display());

    let output = Command::new("tar")
        .arg(tar_extract_flag(&plan.archive)?)
        .arg(&plan.archive)
        .arg("-C")
        .arg(&plan.output_dir)
        .output()
        .with_context(|| "Konnte tar nicht starten. Ist tar im Container installiert?")?;

    check_command_success("Entpackung", "tar", &plan.archive, output)
}

fn tar_test_flag(path: &Path) -> Result<&'static str> {
    let lower = path
        .file_name()
        .map(|name| name.to_string_lossy().to_lowercase())
        .with_context(|| format!("Archiv hat keinen Dateinamen: {}", path.display()))?;

    if lower.ends_with(".tar") {
        return Ok("-tf");
    }

    if lower.ends_with(".tar.gz") || lower.ends_with(".tgz") {
        return Ok("-tzf");
    }

    if lower.ends_with(".tar.xz") || lower.ends_with(".txz") {
        return Ok("-tJf");
    }

    if lower.ends_with(".tar.bz2") || lower.ends_with(".tbz2") {
        return Ok("-tjf");
    }

    bail!("Nicht unterstütztes TAR-Format: {}", path.display())
}

fn tar_extract_flag(path: &Path) -> Result<&'static str> {
    let lower = path
        .file_name()
        .map(|name| name.to_string_lossy().to_lowercase())
        .with_context(|| format!("Archiv hat keinen Dateinamen: {}", path.display()))?;

    if lower.ends_with(".tar") {
        return Ok("-xf");
    }

    if lower.ends_with(".tar.gz") || lower.ends_with(".tgz") {
        return Ok("-xzf");
    }

    if lower.ends_with(".tar.xz") || lower.ends_with(".txz") {
        return Ok("-xJf");
    }

    if lower.ends_with(".tar.bz2") || lower.ends_with(".tbz2") {
        return Ok("-xjf");
    }

    bail!("Nicht unterstütztes TAR-Format: {}", path.display())
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
        || lower == format!("{}.tar.gz", prefix)
        || lower == format!("{}.tgz", prefix)
        || lower == format!("{}.tar.xz", prefix)
        || lower == format!("{}.txz", prefix)
        || lower == format!("{}.tar.bz2", prefix)
        || lower == format!("{}.tbz2", prefix)
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
        || lower.ends_with(".tar.gz")
        || lower.ends_with(".tgz")
        || lower.ends_with(".tar.xz")
        || lower.ends_with(".txz")
        || lower.ends_with(".tar.bz2")
        || lower.ends_with(".tbz2")
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

    for suffix in [
        ".part01.rar",
        ".tar.bz2",
        ".tar.gz",
        ".tar.xz",
        ".tbz2",
        ".tgz",
        ".txz",
        ".rar",
        ".zip",
        ".7z",
        ".tar",
        ".001",
    ] {
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

fn is_tar_archive_path(path: &Path) -> bool {
    let Some(file_name) = path.file_name() else {
        return false;
    };

    let lower = file_name.to_string_lossy().to_lowercase();

    is_tar_archive_name(&lower)
}

fn is_tar_archive_name(file_name: &str) -> bool {
    file_name.ends_with(".tar")
        || file_name.ends_with(".tar.gz")
        || file_name.ends_with(".tgz")
        || file_name.ends_with(".tar.xz")
        || file_name.ends_with(".txz")
        || file_name.ends_with(".tar.bz2")
        || file_name.ends_with(".tbz2")
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
        || lower.ends_with(".tar.gz")
        || lower.ends_with(".tgz")
        || lower.ends_with(".tar.xz")
        || lower.ends_with(".txz")
        || lower.ends_with(".tar.bz2")
        || lower.ends_with(".tbz2")
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn detects_supported_archive_file_names() {
        let supported = [
            "movie.rar",
            "movie.part01.rar",
            "movie.part02.rar",
            "movie.zip",
            "movie.7z",
            "movie.001",
            "movie.r00",
            "movie.tar",
            "movie.tar.gz",
            "movie.tgz",
            "movie.tar.xz",
            "movie.txz",
            "movie.tar.bz2",
            "movie.tbz2",
        ];

        for file in supported {
            assert!(
                is_archive_file_name(file),
                "sollte als Archiv erkannt werden: {}",
                file
            );
        }
    }

    #[test]
    fn ignores_non_archive_file_names() {
        let ignored = [
            "movie.mkv",
            "movie.mp4",
            "subtitle.srt",
            "info.nfo",
            "readme.txt",
            "poster.jpg",
            "sample.avi",
        ];

        for file in ignored {
            assert!(
                !is_archive_file_name(file),
                "sollte nicht als Archiv erkannt werden: {}",
                file
            );
        }
    }

    #[test]
    fn detects_archive_start_files() {
        let starts = [
            "/downloads/movie.rar",
            "/downloads/movie.part01.rar",
            "/downloads/movie.part001.rar",
            "/downloads/movie.zip",
            "/downloads/movie.7z",
            "/downloads/movie.001",
            "/downloads/movie.tar",
            "/downloads/movie.tar.gz",
            "/downloads/movie.tgz",
            "/downloads/movie.tar.xz",
            "/downloads/movie.txz",
            "/downloads/movie.tar.bz2",
            "/downloads/movie.tbz2",
        ];

        for file in starts {
            assert!(
                is_archive_start_file(Path::new(file)),
                "sollte Startarchiv sein: {}",
                file
            );
        }
    }

    #[test]
    fn rejects_non_start_rar_parts() {
        let non_starts = [
            "/downloads/movie.part02.rar",
            "/downloads/movie.part10.rar",
            "/downloads/movie.part999.rar",
        ];

        for file in non_starts {
            assert!(
                !is_archive_start_file(Path::new(file)),
                "sollte kein Startarchiv sein: {}",
                file
            );
        }
    }

    #[test]
    fn strips_archive_extensions_for_release_names() {
        let cases = [
            ("Movie.Name.rar", "Movie.Name"),
            ("Movie.Name.zip", "Movie.Name"),
            ("Movie.Name.7z", "Movie.Name"),
            ("Movie.Name.tar", "Movie.Name"),
            ("Movie.Name.tar.gz", "Movie.Name"),
            ("Movie.Name.tgz", "Movie.Name"),
            ("Movie.Name.tar.xz", "Movie.Name"),
            ("Movie.Name.txz", "Movie.Name"),
            ("Movie.Name.tar.bz2", "Movie.Name"),
            ("Movie.Name.tbz2", "Movie.Name"),
            ("Movie.Name.001", "Movie.Name"),
            ("Movie.Name.part01.rar", "Movie.Name"),
        ];

        for (input, expected) in cases {
            assert_eq!(
                strip_archive_extension(input),
                expected,
                "falscher Release-Name für {}",
                input
            );
        }
    }

    #[test]
    fn detects_tar_archive_names() {
        let tar_files = [
            "backup.tar",
            "backup.tar.gz",
            "backup.tgz",
            "backup.tar.xz",
            "backup.txz",
            "backup.tar.bz2",
            "backup.tbz2",
        ];

        for file in tar_files {
            assert!(
                is_tar_archive_name(file),
                "sollte TAR-Archiv sein: {}",
                file
            );
        }
    }

    #[test]
    fn maps_root_rar_parts_to_first_part() {
        let target = root_archive_target(Path::new("/downloads/movie.part08.rar"))
            .expect("Root target erwartet");

        assert_eq!(target, Path::new("/downloads/movie.part01.rar"));
    }

    #[test]
    fn maps_root_split_parts_to_first_part() {
        let target =
            root_archive_target(Path::new("/downloads/movie.007")).expect("Root target erwartet");

        assert_eq!(target, Path::new("/downloads/movie.001"));
    }

    #[test]
    fn cleanup_group_matches_related_files_only() {
        let prefix = cleanup_group_prefix("Movie.Release.part01.rar");

        assert!(belongs_to_cleanup_group(
            "Movie.Release.part01.rar",
            &prefix
        ));
        assert!(belongs_to_cleanup_group(
            "Movie.Release.part02.rar",
            &prefix
        ));
        assert!(belongs_to_cleanup_group("Movie.Release.rar", &prefix));

        assert!(!belongs_to_cleanup_group("Other.Release.rar", &prefix));
        assert!(!belongs_to_cleanup_group("Movie.Release.mkv", &prefix));
        assert!(!belongs_to_cleanup_group("Movie.Release.srt", &prefix));
    }
}
