use crate::{config::Config, extractor, history::History};
use anyhow::{Context, Result};
use std::{
    collections::BTreeSet,
    env, fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, PartialEq, Eq)]
struct ScanCandidate {
    path: PathBuf,
    state: ScanState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScanState {
    New,
    Done,
    Failed,
}

impl ScanState {
    fn label(self) -> &'static str {
        match self {
            ScanState::New => "new",
            ScanState::Done => "done",
            ScanState::Failed => "failed",
        }
    }
}

pub fn is_scan_command() -> bool {
    env::args().any(|arg| arg == "--scan" || arg == "scan")
}

pub fn run_from_args() -> Result<()> {
    let config_path = crate::status::config_path_from_args();
    let config = Config::load(&config_path)?;

    print_scan(&config)
}

pub fn print_scan(config: &Config) -> Result<()> {
    println!("== XDCC Extractor Scan ==");
    println!();

    println!("Watch directory:");
    println!("  {}", config.watch.directory);
    println!();

    println!("Root-Archive erlaubt:");
    println!("  {}", config.watch.allow_root_archives);
    println!();

    println!("History directory:");
    println!("  {}", config.history.directory);
    println!();

    let candidates = scan_candidates_with_history(config)?;

    println!("Gefundene Kandidaten:");
    println!("  {}", candidates.len());
    println!();

    let mut new_count = 0;
    let mut done_count = 0;
    let mut failed_count = 0;

    if candidates.is_empty() {
        println!("Keine verarbeitbaren Releases gefunden.");
    } else {
        for candidate in &candidates {
            match candidate.state {
                ScanState::New => new_count += 1,
                ScanState::Done => done_count += 1,
                ScanState::Failed => failed_count += 1,
            }

            println!(
                "  [{:<6}] {}",
                candidate.state.label(),
                candidate.path.display()
            );
        }
    }

    println!();
    println!("Zusammenfassung:");
    println!("  new: {}", new_count);
    println!("  done: {}", done_count);
    println!("  failed: {}", failed_count);
    println!();

    println!("Scan abgeschlossen. Es wurde nichts entpackt und nichts gelöscht.");

    Ok(())
}

fn scan_candidates_with_history(config: &Config) -> Result<Vec<ScanCandidate>> {
    let candidates = scan_candidates(config)?;
    let history = History::new(Path::new(&config.history.directory))?;

    let mut result = Vec::new();

    for path in candidates {
        let state = classify_candidate(&history, &path);
        result.push(ScanCandidate { path, state });
    }

    Ok(result)
}

fn classify_candidate(history: &History, path: &Path) -> ScanState {
    if history.is_done(path) {
        return ScanState::Done;
    }

    match history.failed_attempts(path) {
        Ok(attempts) if attempts > 0 => ScanState::Failed,
        _ => ScanState::New,
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
    fn scan_finds_folder_and_root_archive_candidates() {
        let dir = tempdir().expect("tempdir");
        let history_dir = dir.path().join("history");

        let folder_release = dir.path().join("Folder.Release");
        fs::create_dir_all(&folder_release).expect("create folder release");
        fs::write(folder_release.join("Folder.Release.rar"), "rar").expect("write folder rar");

        let root_part1 = dir.path().join("Root.Release.part01.rar");
        let root_part2 = dir.path().join("Root.Release.part02.rar");
        fs::write(&root_part1, "part1").expect("write part1");
        fs::write(&root_part2, "part2").expect("write part2");

        let ignored_dir = dir.path().join("_extracted");
        fs::create_dir_all(&ignored_dir).expect("create ignored dir");
        fs::write(ignored_dir.join("Ignored.Release.rar"), "rar").expect("write ignored rar");

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
                history_dir.display()
            ),
        )
        .expect("write config");

        let config = Config::load(&config_file).expect("load config");
        let candidates = scan_candidates(&config).expect("scan");

        assert!(candidates.contains(&folder_release));
        assert!(candidates.contains(&root_part1));
        assert!(!candidates.contains(&ignored_dir));
    }

    #[test]
    fn scan_ignores_root_archives_when_disabled() {
        let dir = tempdir().expect("tempdir");
        let history_dir = dir.path().join("history");

        let root_archive = dir.path().join("Root.Disabled.zip");
        fs::write(&root_archive, "zip").expect("write zip");

        let config_file = dir.path().join("config.toml");
        fs::write(
            &config_file,
            format!(
                r#"
[watch]
directory="{}"
allow_root_archives=false

[history]
directory="{}"
"#,
                dir.path().display(),
                history_dir.display()
            ),
        )
        .expect("write config");

        let config = Config::load(&config_file).expect("load config");
        let candidates = scan_candidates(&config).expect("scan");

        assert!(candidates.is_empty());
    }

    #[test]
    fn scan_marks_candidates_as_new_done_or_failed() {
        let dir = tempdir().expect("tempdir");
        let history_dir = dir.path().join("history");

        let new_archive = dir.path().join("New.Release.zip");
        let done_archive = dir.path().join("Done.Release.zip");
        let failed_archive = dir.path().join("Failed.Release.zip");

        fs::write(&new_archive, "new").expect("write new");
        fs::write(&done_archive, "done").expect("write done");
        fs::write(&failed_archive, "failed").expect("write failed");

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
                history_dir.display()
            ),
        )
        .expect("write config");

        let config = Config::load(&config_file).expect("load config");
        let history = History::new(&history_dir).expect("history");

        history.mark_done(&done_archive).expect("mark done");
        history
            .mark_failed(&failed_archive, "test error")
            .expect("mark failed");

        let candidates = scan_candidates_with_history(&config).expect("scan");

        let new = candidates
            .iter()
            .find(|candidate| candidate.path == new_archive)
            .expect("new candidate");

        let done = candidates
            .iter()
            .find(|candidate| candidate.path == done_archive)
            .expect("done candidate");

        let failed = candidates
            .iter()
            .find(|candidate| candidate.path == failed_archive)
            .expect("failed candidate");

        assert_eq!(new.state, ScanState::New);
        assert_eq!(done.state, ScanState::Done);
        assert_eq!(failed.state, ScanState::Failed);
    }
}
