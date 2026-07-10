use crate::{config::Config, extractor, history::History};
use anyhow::Result;
use std::{
    env,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CandidateState {
    New,
    Done,
    Failed,
}

impl CandidateState {
    fn label(self) -> &'static str {
        match self {
            CandidateState::New => "new",
            CandidateState::Done => "done",
            CandidateState::Failed => "failed",
        }
    }
}

pub fn is_dry_run_check_command() -> bool {
    env::args().any(|arg| arg == "--dry-run-check" || arg == "dry-run-check")
}

pub fn run_from_args() -> Result<()> {
    let config_path = crate::status::config_path_from_args();
    let config = Config::load(&config_path)?;

    print_check(&config)
}

pub fn print_check(config: &Config) -> Result<()> {
    println!("== XDCC Extractor Dry-Run Safety Check ==");
    println!();

    let watch_dirs = config
        .watch
        .resolved_directories()
        .into_iter()
        .map(Path::new)
        .collect::<Vec<_>>();

    let output_dir = Path::new(&config.output.directory);

    let history_dir = Path::new(&config.history.directory);

    let mut blockers = Vec::new();
    let mut warnings = Vec::new();

    println!("Konfiguration:");
    println!("  watch:");

    for watch_dir in &watch_dirs {
        println!("    {}", watch_dir.display());
    }

    println!("  output: {}", output_dir.display());
    println!("  history: {}", history_dir.display());
    println!("  delete_archives: {}", config.extract.delete_archives);
    println!("  dry_run: {}", config.extract.dry_run);
    println!("  keep_failed: {}", config.extract.keep_failed);
    println!(
        "  allow_root_archives: {}",
        config.watch.allow_root_archives
    );
    println!();

    for watch_dir in &watch_dirs {
        if !watch_dir.is_dir() {
            blockers.push(format!(
                "Watch-Ordner existiert nicht: {}",
                watch_dir.display()
            ));
        }
    }

    if watch_dirs.iter().any(|watch_dir| output_dir == *watch_dir) {
        blockers.push("Output-Ordner darf nicht identisch mit einem Watch-Ordner sein".to_string());
    }

    if !output_dir.exists() {
        warnings.push(format!(
            "Output-Ordner existiert noch nicht: {}",
            output_dir.display()
        ));
    }

    if !history_dir.exists() {
        warnings.push(format!(
            "History-Ordner existiert noch nicht: {}",
            history_dir.display()
        ));
    }

    if !config.extract.delete_archives {
        warnings.push(
            "delete_archives=false: dry_run=false würde trotzdem keine Archive löschen".to_string(),
        );
    }

    if !config.extract.dry_run {
        warnings.push("dry_run=false ist bereits aktiv".to_string());
    }

    let candidates = scan_candidates(config)?;
    let history = History::new(&config.history.directory)?;

    let mut new_count = 0;
    let mut done_count = 0;
    let mut failed_count = 0;
    let mut cleanup_file_count = 0;
    let mut unsafe_cleanup_count = 0;

    println!("Kandidaten:");
    println!("  gesamt: {}", candidates.len());

    for candidate in &candidates {
        let state = classify_candidate(&history, candidate);

        match state {
            CandidateState::New => new_count += 1,
            CandidateState::Done => done_count += 1,
            CandidateState::Failed => failed_count += 1,
        }

        println!("  [{:<6}] {}", state.label(), candidate.display());

        match extractor::create_extract_plan(candidate, output_dir) {
            Ok(plan) => {
                cleanup_file_count += plan.cleanup_files.len();

                for file in &plan.cleanup_files {
                    if !is_safe_cleanup_candidate(&plan.release_root, file) {
                        unsafe_cleanup_count += 1;
                        blockers.push(format!("Unsicherer Cleanup-Kandidat: {}", file.display()));
                    }
                }
            }
            Err(err) => {
                blockers.push(format!(
                    "Konnte Extract-Plan nicht erstellen für {}: {:?}",
                    candidate.display(),
                    err
                ));
            }
        }
    }

    println!();
    println!("Zusammenfassung:");
    println!("  new: {}", new_count);
    println!("  done: {}", done_count);
    println!("  failed: {}", failed_count);
    println!("  Cleanup-Dateien gesamt: {}", cleanup_file_count);
    println!("  Unsichere Cleanup-Dateien: {}", unsafe_cleanup_count);
    println!();

    if new_count > 0 {
        warnings.push(format!(
            "{} neue Kandidat(en) vorhanden. Bei normalem Worker-Lauf und dry_run=false würden Archive nach erfolgreicher Verarbeitung gelöscht.",
            new_count
        ));
    }

    if failed_count > 0 {
        warnings.push(format!(
            "{} fehlgeschlagene Kandidat(en) vorhanden. Vor erneutem Versuch ggf. --clear-failed nutzen.",
            failed_count
        ));
    }

    if cleanup_file_count == 0 {
        warnings.push("Keine Cleanup-Kandidaten gefunden. Es gibt aktuell nichts, womit dry_run=false praktisch getestet würde.".to_string());
    }

    println!("Blocker:");
    if blockers.is_empty() {
        println!("  keine");
    } else {
        for blocker in &blockers {
            println!("  - {}", blocker);
        }
    }

    println!();
    println!("Warnungen:");
    if warnings.is_empty() {
        println!("  keine");
    } else {
        for warning in &warnings {
            println!("  - {}", warning);
        }
    }

    println!();

    if !blockers.is_empty() {
        println!("Bereitschaft für dry_run=false:");
        println!("  NEIN");
    } else if !warnings.is_empty() {
        println!("Bereitschaft für dry_run=false:");
        println!("  BEDINGT");
    } else {
        println!("Bereitschaft für dry_run=false:");
        println!("  JA");
    }

    println!();
    println!("Check abgeschlossen. Es wurde nichts entpackt und nichts gelöscht.");

    Ok(())
}

fn classify_candidate(history: &History, path: &Path) -> CandidateState {
    if history.is_done(path) {
        return CandidateState::Done;
    }

    match history.failed_attempts(path) {
        Ok(attempts) if attempts > 0 => CandidateState::Failed,
        _ => CandidateState::New,
    }
}

fn is_safe_cleanup_candidate(release_root: &Path, file: &Path) -> bool {
    if file.parent() != Some(release_root) {
        return false;
    }

    if !file.is_file() {
        return false;
    }

    extractor::is_archive_related_file(file)
}

fn scan_candidates(config: &Config) -> Result<Vec<PathBuf>> {
    crate::scan::scan_candidate_paths(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn safe_cleanup_candidate_must_be_inside_release_root() {
        let dir = tempdir().expect("tempdir");
        let outside = tempdir().expect("outside");

        let archive = dir.path().join("Movie.Release.rar");
        let outside_archive = outside.path().join("Movie.Release.part02.rar");

        fs::write(&archive, "rar").expect("write archive");
        fs::write(&outside_archive, "rar").expect("write outside archive");

        assert!(is_safe_cleanup_candidate(dir.path(), &archive));
        assert!(!is_safe_cleanup_candidate(dir.path(), &outside_archive));
    }

    #[test]
    fn scan_finds_root_archive_candidate_for_check() {
        let dir = tempdir().expect("tempdir");

        let archive = dir.path().join("Check.Release.zip");
        fs::write(&archive, "zip").expect("write archive");

        let config_file = dir.path().join("config.toml");

        fs::write(
            &config_file,
            format!(
                r#"
[watch]
directory="{}"
allow_root_archives=true

[history]
directory="{}"
"#,
                dir.path().display(),
                dir.path().join("history").display()
            ),
        )
        .expect("write config");

        let config = Config::load(&config_file).expect("load config");
        let candidates = scan_candidates(&config).expect("scan");

        assert_eq!(candidates, vec![archive]);
    }
}
