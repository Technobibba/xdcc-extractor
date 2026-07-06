use crate::{config::Config, extractor, history::History};
use anyhow::{Context, Result};
use std::{
    collections::BTreeSet,
    env, fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReportState {
    New,
    Done,
    Failed,
}

impl ReportState {
    fn label(self) -> &'static str {
        match self {
            ReportState::New => "new",
            ReportState::Done => "done",
            ReportState::Failed => "failed",
        }
    }
}

pub fn is_dry_run_report_command() -> bool {
    env::args().any(|arg| arg == "--dry-run-report" || arg == "dry-run-report")
}

pub fn run_from_args() -> Result<()> {
    let config_path = crate::status::config_path_from_args();
    let config = Config::load(&config_path)?;

    print_report(&config)
}

pub fn print_report(config: &Config) -> Result<()> {
    println!("== XDCC Extractor Dry-Run Report ==");
    println!();

    println!("Konfiguration:");
    println!("  watch: {}", config.watch.directory);
    println!("  output: {}", config.output.directory);
    println!("  history: {}", config.history.directory);
    println!(
        "  allow_root_archives: {}",
        config.watch.allow_root_archives
    );
    println!("  delete_archives: {}", config.extract.delete_archives);
    println!("  dry_run: {}", config.extract.dry_run);
    println!("  keep_failed: {}", config.extract.keep_failed);
    println!();

    if !config.extract.dry_run {
        println!("WARNUNG:");
        println!("  dry_run=false ist aktuell gesetzt.");
        println!(
            "  Dieser Report löscht trotzdem nichts, aber der normale Worker würde Cleanup ausführen."
        );
        println!();
    }

    let candidates = scan_candidates(config)?;
    let history = History::new(&config.history.directory)?;

    let mut new_count = 0;
    let mut done_count = 0;
    let mut failed_count = 0;
    let mut cleanup_file_count = 0;

    println!("Gefundene Kandidaten:");
    println!("  {}", candidates.len());
    println!();

    if candidates.is_empty() {
        println!("Keine Kandidaten gefunden.");
    }

    for candidate in &candidates {
        let state = classify_candidate(&history, candidate);

        match state {
            ReportState::New => new_count += 1,
            ReportState::Done => done_count += 1,
            ReportState::Failed => failed_count += 1,
        }

        println!("[{}] {}", state.label(), candidate.display());

        match extractor::create_extract_plan(candidate, Path::new(&config.output.directory)) {
            Ok(plan) => {
                println!("  Archiv:");
                println!("    {}", plan.archive.display());
                println!("  Zielordner:");
                println!("    {}", plan.output_dir.display());
                println!("  Cleanup-Kandidaten:");
                println!("    {}", plan.cleanup_files.len());

                cleanup_file_count += plan.cleanup_files.len();

                for file in &plan.cleanup_files {
                    println!("    - {}", file.display());
                }

                if config.extract.delete_archives && config.extract.dry_run {
                    println!("  Ergebnis:");
                    println!("    Archive würden bei dry_run=false gelöscht.");
                } else if config.extract.delete_archives && !config.extract.dry_run {
                    println!("  Ergebnis:");
                    println!("    Normaler Worker würde Archive nach Erfolg löschen.");
                } else {
                    println!("  Ergebnis:");
                    println!("    Cleanup ist deaktiviert.");
                }
            }
            Err(err) => {
                println!("  Plan-Fehler:");
                println!("    {:?}", err);
            }
        }

        println!();
    }

    println!("Zusammenfassung:");
    println!("  new: {}", new_count);
    println!("  done: {}", done_count);
    println!("  failed: {}", failed_count);
    println!("  Cleanup-Dateien gesamt: {}", cleanup_file_count);
    println!();

    println!("Report abgeschlossen. Es wurde nichts entpackt und nichts gelöscht.");

    Ok(())
}

fn classify_candidate(history: &History, path: &Path) -> ReportState {
    if history.is_done(path) {
        return ReportState::Done;
    }

    match history.failed_attempts(path) {
        Ok(attempts) if attempts > 0 => ReportState::Failed,
        _ => ReportState::New,
    }
}

fn scan_candidates(config: &Config) -> Result<Vec<PathBuf>> {
    let watch_dir = Path::new(&config.watch.directory);

    if !watch_dir.is_dir() {
        anyhow::bail!("Watch directory existiert nicht: {}", watch_dir.display());
    }

    let mut candidates = BTreeSet::new();

    for entry in fs::read_dir(watch_dir)
        .with_context(|| format!("Konnte Watch-Ordner nicht lesen: {}", watch_dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();

        if should_skip_entry(&name) {
            continue;
        }

        if path.is_dir() {
            if extractor::has_archive_start(&path)? {
                candidates.insert(path);
            }

            continue;
        }

        if path.is_file()
            && config.watch.allow_root_archives
            && extractor::is_archive_related_file(&path)
        {
            if let Some(target) = extractor::root_archive_target(&path) {
                if target.exists() && extractor::has_archive_start(&target)? {
                    candidates.insert(target);
                }
            }
        }
    }

    Ok(candidates.into_iter().collect())
}

fn should_skip_entry(name: &str) -> bool {
    matches!(
        name,
        "_extracted" | "_failed" | "_processing" | ".xdcc-worker"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn report_scan_finds_root_archive_candidate() {
        let dir = tempdir().expect("tempdir");

        let root_archive = dir.path().join("Report.Test.zip");
        fs::write(&root_archive, "zip").expect("write zip");

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

        assert_eq!(candidates, vec![root_archive]);
    }

    #[test]
    fn report_classifies_done_and_failed_candidates() {
        let dir = tempdir().expect("tempdir");
        let history = History::new(dir.path().join("history")).expect("history");

        let new_release = dir.path().join("New.Release.zip");
        let done_release = dir.path().join("Done.Release.zip");
        let failed_release = dir.path().join("Failed.Release.zip");

        assert_eq!(classify_candidate(&history, &new_release), ReportState::New);

        history.mark_done(&done_release).expect("mark done");
        history
            .mark_failed(&failed_release, "test error")
            .expect("mark failed");

        assert_eq!(
            classify_candidate(&history, &done_release),
            ReportState::Done
        );
        assert_eq!(
            classify_candidate(&history, &failed_release),
            ReportState::Failed
        );
    }
}
